import React from 'react';
function ForwardedAction() {
    return <div data-component="ForwardedAction" data-source-file="input.jsx" data-source-path="/mock/absolute/path/tests/fixture/react_forwarded_props_precedence/input.jsx">
      <ForwardedButton data-element="ForwardedButton" data-source-file="input.jsx" data-source-path="/mock/absolute/path/tests/fixture/react_forwarded_props_precedence/input.jsx"/>
      <ForwardedPanel role="region" data-element="ForwardedPanel" data-source-file="input.jsx" data-source-path="/mock/absolute/path/tests/fixture/react_forwarded_props_precedence/input.jsx"/>
      <ForwardedInput aria-label="Search" data-element="ForwardedInput" data-source-file="input.jsx" data-source-path="/mock/absolute/path/tests/fixture/react_forwarded_props_precedence/input.jsx"/>
    </div>;
}
function ForwardedButton(props) {
    return <button data-component="ForwardedButton" data-source-file="input.jsx" data-source-path="/mock/absolute/path/tests/fixture/react_forwarded_props_precedence/input.jsx" {...props}>Forwarded</button>;
}
function ForwardedPanel(props) {
    return <div className="panel" data-component="ForwardedPanel" data-source-file="input.jsx" data-source-path="/mock/absolute/path/tests/fixture/react_forwarded_props_precedence/input.jsx" {...props}>Panel</div>;
}
function ForwardedInput(props) {
    return <input type="text" data-component="ForwardedInput" data-source-file="input.jsx" data-source-path="/mock/absolute/path/tests/fixture/react_forwarded_props_precedence/input.jsx" {...props}/>;
}
