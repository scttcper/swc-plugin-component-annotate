import React from 'react';

// This component should be ignored
function IgnoredComponent() {
  return <div>This should not have attributes</div>;
}

// This component should be annotated
function RegularComponent() {
  return <div>This should have attributes</div>;
}

// This component should also be ignored
const AnotherIgnoredComponent = () => {
  return <span>Also ignored</span>;
};

// This component should be annotated
const AnotherRegularComponent = () => {
  return <p>Should be annotated</p>;
};

// Class component that should be ignored
class IgnoredClassComponent extends React.Component {
  render() {
    return <section>Ignored class component</section>;
  }
}

// Class component that should be annotated
class RegularClassComponent extends React.Component {
  render() {
    return <article>Regular class component</article>;
  }
} 