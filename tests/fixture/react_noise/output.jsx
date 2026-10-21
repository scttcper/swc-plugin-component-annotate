import React from 'react';
// Lowercase helpers that return JSX are not components
function renderRow(item) {
    return <Row label={item.label} data-element="Row" data-source-file="test.jsx"/>;
}
const getIcon = ()=><Icon data-element="Icon" data-source-file="test.jsx"/>;
function List({ items }) {
    return <ul data-component="List" data-source-file="test.jsx">{items.map(renderRow)}</ul>;
}
// Polymorphic tags name a local, not what rendered
function Polymorphic({ as, ...props }) {
    const Component = as ?? 'div';
    return <Component {...props} data-component="Polymorphic" data-source-file="test.jsx">
      <Component/>
    </Component>;
}
// Members of local values are dropped, component namespaces are kept
function Toggle({ field }) {
    return <div data-component="Toggle" data-source-file="test.jsx">
      <field.Switch label="Enabled"/>
      <Fields.Switch label="Enabled" data-element="Fields.Switch" data-source-file="test.jsx"/>
    </div>;
}
