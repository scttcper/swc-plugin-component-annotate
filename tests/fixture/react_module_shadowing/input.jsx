export function ModuleFragmentShadowBeforeDeclaration({children}) {
  return <Fragment>{children}</Fragment>;
}

export function ModuleNamespaceShadowBeforeDeclaration({children}) {
  return <React.Fragment>{children}</React.Fragment>;
}

const Fragment = ({children}) => <article>{children}</article>;

const React = {
  Fragment: ({children}) => <section>{children}</section>,
};

export function ModuleFragmentShadow({children}) {
  return <Fragment>{children}</Fragment>;
}

export function ModuleNamespaceShadow({children}) {
  return <React.Fragment>{children}</React.Fragment>;
}
