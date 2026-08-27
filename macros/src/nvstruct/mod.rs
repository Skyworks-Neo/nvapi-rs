use {
    crate::{inherit::NvInheritArgs, prelude::*, version::NvVersionArgs},
    syn::{TypeArray, punctuated::Punctuated},
};

mod align;

pub use self::align::NvAlignArgs;

pub fn NvStruct(attr: TokenStream, input: TokenStream) -> Result<TokenStream> {
    if !attr.is_empty() {
        return Err(call_error("#[NvStruct] takes no arguments"));
    }
    let item = input.clone();
    let mut item: DeriveStruct = parse(item)?;

    let repr = match item.attrs.iter().any(|a| a.path().is_ident("repr")) {
        true => quote!(),
        false => quote! {
            #[repr(C)]
        },
    };

    // surface malformed `#[derive(..)]` lists as spanned compile errors
    // instead of silently ignoring them in `has_derive` below
    for attr in &item.attrs {
        attr_derives(attr)?;
    }

    let has_derive = |derive: &'static str| {
        |a: &Attribute| {
            attr_derives(a)
                .ok()
                .flatten()
                .map(|derives| {
                    derives
                        .iter()
                        .any(|path| path_tail_is(derive, &path.segments))
                })
                .unwrap_or(false)
        }
    };

    let has_version =
        item.data().fields.iter().any(
            |f| matches!(&f.ty, Type::Path(ty) if path_tail_is("NvVersion", &ty.path.segments)),
        );
    let derives = match has_version {
        true if !item.attrs.iter().any(has_derive("VersionedStructField")) => {
            Some("VersionedStructField")
        }
        _ => None,
    };

    let derives: Punctuated<Path, Token![,]> = {
        // add missing derives if their related field attributes are found
        const ATTR_DERIVES: [(&str, &str); 2] = [
            (NvVersionArgs::NAME, "VersionedStructField"),
            (NvInheritArgs::NAME, "NvInherit"),
        ];

        ATTR_DERIVES
            .iter()
            .filter(|(attr_ident, derive)| {
                let has_attr = item
                    .data()
                    .fields
                    .iter()
                    .any(|f| f.attrs.iter().any(|a| a.path().is_ident(attr_ident)));
                has_attr && !item.attrs.iter().any(has_derive(derive))
            })
            .map(|&(_, derive)| derive)
            .chain(derives)
            .map(|derive| Path::from(call_ident(derive)))
            .collect()
    };

    // `#[nv_unchecked]`: structs with fields zerocopy's derive rejects
    // (function pointers, etc). Skip the derive and emit the manual
    // unsafe impls instead — same semantics as the pre-proc-macro macro_rules.
    let unchecked = item.attrs.iter().any(|a| a.path().is_ident("nv_unchecked"));
    if unchecked {
        if let Some(i) = item
            .attrs
            .iter()
            .position(|a| a.path().is_ident("nv_unchecked"))
        {
            item.attrs.remove(i);
        }
    }

    // clone: `name` is used inside the padding-rewrite loop below, where
    // `item` is mutably borrowed by `data_mut()`
    let name = item.ident.clone();
    let AsBytes = call_path_absolute(["zerocopy", "AsBytes"]);
    let FromBytes = call_path_absolute(["zerocopy", "FromBytes"]);

    let (struct_attrs, expanded) = match unchecked {
        false => (
            quote! {
                #[derive(Copy, Clone, Debug, #AsBytes, #FromBytes, #derives)]
                #repr
            },
            quote! {
                impl #name {
                    /// Returns a zero-filled instance.
                    ///
                    /// # Stack hazard
                    ///
                    /// Constructs by value on the caller's stack. For structs
                    /// near or above 1 MiB (VF-points family and friends) use
                    /// `Box::<Self>::new_zeroed()` instead — see the
                    /// `Box::new(Default)` debug stack-overflow incident.
                    pub fn zeroed() -> Self {
                        #FromBytes::new_zeroed()
                    }
                }
            },
        ),
        true => (
            quote! {
                #[derive(Copy, Clone, Debug, #derives)]
                #repr
            },
            quote! {
                impl #name {
                    /// Returns a zero-filled instance.
                    ///
                    /// # Stack hazard
                    ///
                    /// Constructs by value on the caller's stack. For structs
                    /// near or above 1 MiB use `Box::<Self>::new_zeroed()`
                    /// instead. The all-zero bit pattern is asserted valid for
                    /// this struct via `#[nv_unchecked]`.
                    #[inline]
                    pub fn zeroed() -> Self {
                        let mut zero = ::core::mem::MaybeUninit::<Self>::zeroed();
                        unsafe { zero.assume_init() }
                    }
                }

                unsafe impl #AsBytes for #name {
                    fn only_derive_is_allowed_to_implement_this_trait() where Self: Sized { }
                }

                unsafe impl #FromBytes for #name {
                    fn only_derive_is_allowed_to_implement_this_trait() where Self: Sized { }
                }
            },
        ),
    };

    let padding_fields = item
        .data()
        .fields
        .iter()
        .enumerate()
        .flat_map(|(field_index, field)| {
            field.attrs.iter().filter_map(move |attr| {
                try_parse_attr::<NvAlignArgs>(attr)
                    .transpose()
                    .map(|a| a.map(|attr| (field_index, attr)))
            })
        })
        .collect::<Result<Vec<_>>>()?;

    // more than one #[nv_align] on the same field is ambiguous; the old
    // index-based attr removal would also corrupt unrelated attributes
    {
        let mut seen = std::collections::BTreeSet::new();
        for (field_index, align) in &padding_fields {
            if !seen.insert(*field_index) {
                return Err(error(Some(align.span()), "duplicate #[nv_align] attribute"));
            }
        }
    }

    let requires_rewrite = !padding_fields.is_empty() || unchecked;

    let mut align_asserts: TokenStream = quote!();

    let input = match requires_rewrite {
        false => input,
        true => {
            for (field_index, align) in padding_fields {
                let field = item.data_mut().fields.iter_mut().nth(field_index).unwrap();
                // remove by predicate: indices captured before any removal are
                // invalidated by Vec::remove
                field
                    .attrs
                    .retain(|a| !a.path().is_ident(NvAlignArgs::NAME));
                match &mut field.ty {
                    Type::Array(TypeArray { len, elem, .. }) => {
                        let bit_align = align.bit_align;
                        let next_ty = align.ty.as_ref().unwrap();
                        let mem = quote! { ::core::mem };
                        *len = parse_quote! {
                            #mem::align_of::<#next_ty>().saturating_sub(#bit_align / #mem::size_of::<#elem>() / 8)
                        };

                        // `bit_align` is a hand-computed bit offset the padding
                        // length derives from — nothing checks it once fields
                        // change. Pin it to reality at compile time: the padding
                        // field's own offset, taken modulo the NEXT field's
                        // alignment, must equal `bit_align / 8` bytes (this is
                        // exactly the misalignment the padding exists to fix).
                        if let Some(field_ident) = &field.ident {
                            align_asserts.extend(quote! {
                                const _: () = assert!(
                                    #mem::offset_of!(#name, #field_ident) % #mem::align_of::<#next_ty>()
                                        == #bit_align / #mem::size_of::<#elem>() / 8,
                                    "stale #[nv_align] offset: padding field is no longer where the magic number says it is",
                                );
                                const _: () = assert!(
                                    #mem::size_of::<#elem>() == 1,
                                    "alignment padding must be a byte array",
                                );
                            });
                        }
                    }
                    _ => {
                        return Err(Error::new_spanned(
                            field,
                            "alignment padding must be a byte array",
                        ));
                    }
                }
            }

            item.into_token_stream()
        }
    };

    let mut expanded = expanded;
    expanded.extend(align_asserts);

    Ok(struct_attrs
        .into_iter()
        .chain(input)
        .chain(expanded)
        .collect())
}
