use crate::prelude::*;
use syn::{LitInt, parenthesized, token::Paren};

pub struct NvVersionArgs {
    span: Span,
}

impl ContextualAttr for NvVersionArgs {
    const NAME: &'static str = "nv_version_field";
    const HAS_ARGS: bool = false;

    fn span(&self) -> Span {
        self.span
    }

    fn default_with_span(span: Span) -> Result<Self> {
        Ok(Self { span })
    }
}

impl Parse for NvVersionArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let span = input.span();
        let _: ParseEof = input.parse()?;
        Ok(Self { span })
    }
}

impl AddAssign for NvVersionArgs {
    fn add_assign(&mut self, rhs: Self) {
        self.span = self.span.join(rhs.span).unwrap_or(self.span);
    }
}

pub fn derive_versioned_struct(input: TokenStream) -> Result<TokenStream> {
    let input: DeriveStruct = parse(input)?;

    let version_fields: Result<Vec<_>> = input
        .data()
        .fields
        .iter()
        .enumerate()
        .flat_map(|(field_index, field)| {
            let attr = get_field_attr::<NvVersionArgs>(field)
                .transpose()
                .map(move |attr| attr.map(move |attr| (Some(attr), field_index, field)));
            let implicit = match &field.ty {
                Type::Path(ty)
                    if ty
                        .path
                        .segments
                        .last()
                        .map(|id| id.ident == "NvVersion")
                        .unwrap_or(false) =>
                {
                    Some(Ok((None, field_index, field)))
                }
                _ => None,
            };
            attr.into_iter().chain(implicit)
        })
        .collect();

    let version_field_id = match version_fields {
        Err(e) => Err(e),
        Ok(version_fields)
            if version_fields
                .iter()
                .filter(|(attr, ..)| attr.is_some())
                .count()
                > 1 =>
        {
            let last = version_fields
                .iter()
                .filter_map(|(attr, ..)| attr.as_ref())
                .next_back();
            Err(error(
                last.map(|attr| attr.span()),
                "multiple fields specified",
            ))
        }
        Ok(mut version_fields) => {
            version_fields.sort_by_key(|(attr, ..)| attr.is_none());
            match version_fields
                .first()
                .map(|(_attr, i, f)| (i, f.ident.as_ref()))
            {
                Some((_, Some(id))) => Ok(id.to_token_stream()),
                Some((i, None)) => Ok(i.into_token_stream()),
                None => Err(call_error(format_args!(
                    "#[{}] missing",
                    NvVersionArgs::NAME
                ))),
            }
        }
    };

    let name = &input.ident;
    let VersionedStructField = sys_path(["nvapi", "VersionedStructField"]);
    let NvVersion = sys_path(["nvapi", "NvVersion"]);

    let (body, body_mut) = match version_field_id {
        Ok(version_field_id) => (
            quote! {
                #VersionedStructField::nvapi_version_ref(&self.#version_field_id)
            },
            quote! {
                #VersionedStructField::nvapi_version_mut(&mut self.#version_field_id)
            },
        ),
        Err(err) => (err.to_compile_error(), err.into_compile_error()),
    };

    Ok(quote! {
        impl #VersionedStructField for #name {
            fn nvapi_version_ref(&self) -> &#NvVersion {
                #body
            }

            fn nvapi_version_mut(&mut self) -> &mut #NvVersion {
                #body_mut
            }
        }
    })
}

/// Input shape (v0.2.x `nvversion!` syntax, kept verbatim so ~137 call sites
/// need zero changes):
///
/// * `nvversion! { Target(3) }` — emit `StructVersion<3>` impl (+ size assert)
/// * `nvversion! { Target(3) = 128 }` — … and assert `size_of::<Target>() == 128`
/// * `nvversion! { = Alias Target(3) … }` — … and `pub type Alias = Target;`
/// * `nvversion! { @… }` — additionally emit the unversioned `StructVersion`
///   (VER = 0) alias impl + `Default` pinning this version as the struct's
///   default, matching the legacy `macro_rules!` `@` arm byte for byte.
///
/// This deliberately deviates from the donor's `Name: A(1), B(2) = size`
/// family syntax: the donor's `Default` resolves to the *oldest* declared
/// version, while v0.2.x's `@` arm resolves to the *marked* (latest) version.
/// Flipping that would silently change which struct version the FFI boundary
/// sends by default.
pub struct NvVersionBody {
    pub at: Option<Token![@]>,
    pub alias: Option<(Token![=], Ident)>,
    pub target: Ident,
    // stored to consume tokens during Parse but never read (donor parity)
    #[allow(dead_code)]
    pub paren: Paren,
    pub version: LitInt,
    /// `= <size>` trailing assertion, asserts `size_of::<T>() == size` in bytes
    /// (the donor instead asserts the size encoded in `NvVersion`; v0.2.x has
    /// always asserted the real `size_of`).
    pub size: Option<(Token![=], LitInt)>,
}

impl Parse for NvVersionBody {
    fn parse(input: ParseStream) -> Result<Self> {
        let at = input.parse()?;
        let alias = if input.peek(Token![=]) {
            let eq = input.parse()?;
            let ident = input.parse()?;
            Some((eq, ident))
        } else {
            None
        };
        let target = input.parse()?;
        let content;
        let paren = parenthesized!(content in input);
        let version: LitInt = content.parse()?;
        let _: u16 = version.base10_parse()?;
        let _: ParseEof = content.parse()?;
        let size = if input.peek(Token![=]) {
            let eq = input.parse()?;
            let size = input.parse()?;
            Some((eq, size))
        } else {
            None
        };
        let _: ParseEof = input.parse()?;
        Ok(Self {
            at,
            alias,
            target,
            paren,
            version,
            size,
        })
    }
}

impl NvVersionBody {
    pub fn output(&self) -> TokenStream {
        let Self {
            at,
            alias,
            target,
            version,
            size,
            ..
        } = self;

        let StructVersion = sys_path(["nvapi", "StructVersion"]);
        let NvVersion = sys_path(["nvapi", "NvVersion"]);

        let mut expanded = TokenStream::new();

        if let Some((_, name)) = alias {
            expanded.extend(quote! {
                pub type #name = #target;
            });
        }

        expanded.extend(quote! {
            impl #StructVersion<#version> for #target {
                const NVAPI_VERSION: #NvVersion = #NvVersion::with_struct::<#target>(#version);
            }
        });

        if let Some((_, size)) = size {
            expanded.extend(quote! {
                const _: () = assert!(#size == ::core::mem::size_of::<#target>());
            });
        }

        if at.is_some() {
            // legacy `@` arm: unversioned alias impl + Default pinned to this version
            expanded.extend(quote! {
                impl #StructVersion for #target {
                    const NVAPI_VERSION: #NvVersion =
                        <#target as #StructVersion<{ #version }>>::NVAPI_VERSION;

                    fn versioned() -> Self {
                        <#target as #StructVersion<{ #version }>>::versioned()
                    }
                }

                impl ::core::default::Default for #target {
                    fn default() -> Self {
                        #StructVersion::<0>::versioned()
                    }
                }
            });
        }

        expanded
    }
}

pub fn nvversion(input: TokenStream) -> Result<TokenStream> {
    let body: NvVersionBody = parse(input)?;
    Ok(body.output())
}
