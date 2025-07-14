import React from 'react';
import { Tab } from '@headlessui/react';
import { Components } from 'my-ui-library';

const MemberExpressionComponent = () => {
  return (
    <div>
      {/* Simple member expressions */}
      <Tab.Group>
        <Tab.List>
          <Tab>Tab 1</Tab>
          <Tab>Tab 2</Tab>
        </Tab.List>
        <Tab.Panels>
          <Tab.Panel>Content 1</Tab.Panel>
          <Tab.Panel>Content 2</Tab.Panel>
        </Tab.Panels>
      </Tab.Group>

      {/* Nested member expressions */}
      <Components.UI.Button>Click me</Components.UI.Button>
      <Components.UI.Card>
        <Components.UI.Card.Header>Title</Components.UI.Card.Header>
        <Components.UI.Card.Body>Content</Components.UI.Card.Body>
      </Components.UI.Card>

      {/* React.Fragment member expression */}
      <React.Fragment>
        <h1>Inside React.Fragment</h1>
        <p>This should not be annotated</p>
      </React.Fragment>
    </div>
  );
};

export default MemberExpressionComponent; 