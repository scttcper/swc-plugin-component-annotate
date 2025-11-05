use std::borrow::Cow;
use swc_core::ecma::ast::*;

/// Check if a JSX element is a React Fragment
#[inline]
pub fn is_react_fragment(element: &JSXElementName) -> bool {
    match element {
        JSXElementName::Ident(ident) => ident.sym.as_ref() == "Fragment",
        JSXElementName::JSXMemberExpr(member_expr) => {
            // Check for React.Fragment
            if let JSXObject::Ident(obj) = &member_expr.obj {
                if obj.sym.as_ref() == "React" {
                    return member_expr.prop.sym.as_ref() == "Fragment";
                }
            }
            false
        }
        _ => false,
    }
}

/// Extract the element name from a JSX element
#[inline]
pub fn get_element_name(element: &JSXElementName) -> Cow<str> {
    match element {
        JSXElementName::Ident(ident) => Cow::Borrowed(ident.sym.as_ref()),
        JSXElementName::JSXMemberExpr(member_expr) => {
            Cow::Owned(get_member_expression_name(member_expr))
        }
        JSXElementName::JSXNamespacedName(namespaced) => {
            Cow::Owned(format!("{}:{}", namespaced.ns.sym, namespaced.name.sym))
        }
    }
}

/// Recursively build the name for member expressions (e.g., "Components.UI.Button")
fn get_member_expression_name(member_expr: &JSXMemberExpr) -> String {
    let obj_name = match &member_expr.obj {
        JSXObject::Ident(ident) => ident.sym.as_ref(),
        JSXObject::JSXMemberExpr(nested_member) => {
            return format!(
                "{}.{}",
                get_member_expression_name(nested_member),
                member_expr.prop.sym
            );
        }
    };

    format!("{}.{}", obj_name, member_expr.prop.sym)
}

/// Check if a JSX element already has an attribute with the given name
#[inline]
pub fn has_attribute(element: &JSXOpeningElement, attr_name: &str) -> bool {
    element.attrs.iter().any(|attr| {
        matches!(attr, JSXAttrOrSpread::JSXAttr(jsx_attr) 
            if matches!(&jsx_attr.name, JSXAttrName::Ident(ident) 
                if ident.sym.as_ref() == attr_name))
    })
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
