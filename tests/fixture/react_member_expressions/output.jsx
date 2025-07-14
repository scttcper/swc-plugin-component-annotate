import React from 'react';
import { Tab } from '@headlessui/react';
import { Components } from 'my-ui-library';
const MemberExpressionComponent = ()=>{
    return <div data-component="MemberExpressionComponent" data-source-file="test.jsx">
      { /* Simple member expressions */ }
      <Tab.Group data-element="Tab.Group" data-source-file="test.jsx">
        <Tab.List data-element="Tab.List" data-source-file="test.jsx">
          <Tab data-element="Tab" data-source-file="test.jsx">Tab 1</Tab>
          <Tab data-element="Tab" data-source-file="test.jsx">Tab 2</Tab>
        </Tab.List>
        <Tab.Panels data-element="Tab.Panels" data-source-file="test.jsx">
          <Tab.Panel data-element="Tab.Panel" data-source-file="test.jsx">Content 1</Tab.Panel>
          <Tab.Panel data-element="Tab.Panel" data-source-file="test.jsx">Content 2</Tab.Panel>
        </Tab.Panels>
      </Tab.Group>

      { /* Nested member expressions */ }
      <Components.UI.Button data-element="Components.UI.Button" data-source-file="test.jsx">Click me</Components.UI.Button>
      <Components.UI.Card data-element="Components.UI.Card" data-source-file="test.jsx">
        <Components.UI.Card.Header data-element="Components.UI.Card.Header" data-source-file="test.jsx">Title</Components.UI.Card.Header>
        <Components.UI.Card.Body data-element="Components.UI.Card.Body" data-source-file="test.jsx">Content</Components.UI.Card.Body>
      </Components.UI.Card>

      { /* React.Fragment member expression */ }
      <React.Fragment>
        <h1>Inside React.Fragment</h1>
        <p>This should not be annotated</p>
      </React.Fragment>
    </div>;
};
export default MemberExpressionComponent; 