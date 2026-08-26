use std::borrow::Cow;
use swc_core::ecma::ast::*;

/// Extract the element name from a JSX element
#[inline]
pub fn get_element_name(element: &JSXElementName) -> Cow<'_, str> {
    match element {
        JSXElementName::Ident(ident) => Cow::Borrowed(ident.sym.as_ref()),
        JSXElementName::JSXMemberExpr(member_expr) => {
            Cow::Owned(get_member_expression_name(member_expr))
        }
        JSXElementName::JSXNamespacedName(namespaced) => {
            Cow::Owned(format!("{}:{}", namespaced.ns.sym, namespaced.name.sym))
        }
        #[cfg(swc_ast_unknown)]
        _ => panic!("unknown jsx element name"),
    }
}

/// Recursively build the name for member expressions (e.g., "Components.UI.Button")
fn get_member_expression_name(member_expr: &JSXMemberExpr) -> String {
    fn member_expression_name_len(member_expr: &JSXMemberExpr) -> usize {
        let obj_len = match &member_expr.obj {
            JSXObject::Ident(ident) => ident.sym.len(),
            JSXObject::JSXMemberExpr(nested_member) => member_expression_name_len(nested_member),
            #[cfg(swc_ast_unknown)]
            _ => panic!("unknown jsx object"),
        };

        obj_len + 1 + member_expr.prop.sym.len()
    }

    fn push_member_expression_name(target: &mut String, member_expr: &JSXMemberExpr) {
        match &member_expr.obj {
            JSXObject::Ident(ident) => target.push_str(ident.sym.as_ref()),
            JSXObject::JSXMemberExpr(nested_member) => {
                push_member_expression_name(target, nested_member);
            }
            #[cfg(swc_ast_unknown)]
            _ => panic!("unknown jsx object"),
        }

        target.push('.');
        target.push_str(member_expr.prop.sym.as_ref());
    }

    let mut output = String::with_capacity(member_expression_name_len(member_expr));
    push_member_expression_name(&mut output, member_expr);
    output
}

#[derive(Default)]
pub struct AttributePresence {
    pub component: bool,
    pub element: bool,
    pub source_file: bool,
    pub source_path: bool,
}

pub fn attribute_presence(
    element: &JSXOpeningElement,
    component_attr: &IdentName,
    element_attr: &IdentName,
    source_file_attr: &IdentName,
    source_path_attr: Option<&IdentName>,
) -> AttributePresence {
    let mut presence = AttributePresence::default();

    for attr in &element.attrs {
        let JSXAttrOrSpread::JSXAttr(jsx_attr) = attr else {
            continue;
        };
        let JSXAttrName::Ident(ident) = &jsx_attr.name else {
            continue;
        };

        if ident.sym == component_attr.sym {
            presence.component = true;
        } else if ident.sym == element_attr.sym {
            presence.element = true;
        } else if ident.sym == source_file_attr.sym {
            presence.source_file = true;
        } else if source_path_attr.is_some_and(|source_path_attr| ident.sym == source_path_attr.sym)
        {
            presence.source_path = true;
        }
    }

    presence
}

/// Create a JSX attribute with a string value
#[inline]
pub fn create_jsx_attr(name: &str, value: &str) -> JSXAttrOrSpread {
    JSXAttrOrSpread::JSXAttr(JSXAttr {
        span: Default::default(),
        name: JSXAttrName::Ident(IdentName::new(name.into(), Default::default())),
        value: Some(JSXAttrValue::Str(Str {
            span: Default::default(),
            value: value.into(),
            raw: None,
        })),
    })
}

#[inline]
pub fn create_jsx_attr_with_ident(name: &IdentName, value: &str) -> JSXAttrOrSpread {
    JSXAttrOrSpread::JSXAttr(JSXAttr {
        span: Default::default(),
        name: JSXAttrName::Ident(name.clone()),
        value: Some(JSXAttrValue::Str(Str {
            span: Default::default(),
            value: value.into(),
            raw: None,
        })),
    })
}

#[inline]
pub fn create_jsx_attr_with_ident_and_str(name: &IdentName, value: &Str) -> JSXAttrOrSpread {
    JSXAttrOrSpread::JSXAttr(JSXAttr {
        span: Default::default(),
        name: JSXAttrName::Ident(name.clone()),
        value: Some(JSXAttrValue::Str(value.clone())),
    })
}
