import React from 'react';

// Lowercase helpers that return JSX are not components
function renderRow(item) {
  return <Row label={item.label} />;
}

const getIcon = () => <Icon />;

function List({items}) {
  return <ul>{items.map(renderRow)}</ul>;
}

// Polymorphic tags name a local, not what rendered
function Polymorphic({as, ...props}) {
  const Component = as ?? 'div';
  return (
    <Component {...props}>
      <Component />
    </Component>
  );
}

// Members of local values are dropped, component namespaces are kept
function Toggle({field}) {
  return (
    <div>
      <field.Switch label="Enabled" />
      <Fields.Switch label="Enabled" />
    </div>
  );
}
