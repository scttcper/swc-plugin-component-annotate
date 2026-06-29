import React from 'react';

function ForwardedAction() {
  return (
    <div>
      <ForwardedButton />
      <ForwardedPanel role="region" />
      <ForwardedInput aria-label="Search" />
    </div>
  );
}

function ForwardedButton(props) {
  return <button {...props}>Forwarded</button>;
}

function ForwardedPanel(props) {
  return <div className="panel" {...props}>Panel</div>;
}

function ForwardedInput(props) {
  return <input type="text" {...props} />;
}
