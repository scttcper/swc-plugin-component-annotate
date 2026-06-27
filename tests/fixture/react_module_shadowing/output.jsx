export function ModuleFragmentShadowBeforeDeclaration({ children }) {
    return <Fragment data-element="Fragment" data-component="ModuleFragmentShadowBeforeDeclaration" data-source-file="test.jsx">{children}</Fragment>;
}
export function ModuleNamespaceShadowBeforeDeclaration({ children }) {
    return <React.Fragment data-element="React.Fragment" data-component="ModuleNamespaceShadowBeforeDeclaration" data-source-file="test.jsx">{children}</React.Fragment>;
}
const Fragment = ({ children })=><article data-component="Fragment" data-source-file="test.jsx">{children}</article>;
const React = {
    Fragment: ({ children })=><section>{children}</section>
};
export function ModuleFragmentShadow({ children }) {
    return <Fragment data-element="Fragment" data-component="ModuleFragmentShadow" data-source-file="test.jsx">{children}</Fragment>;
}
export function ModuleNamespaceShadow({ children }) {
    return <React.Fragment data-element="React.Fragment" data-component="ModuleNamespaceShadow" data-source-file="test.jsx">{children}</React.Fragment>;
}
