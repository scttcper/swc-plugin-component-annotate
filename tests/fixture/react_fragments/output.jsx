import React, { Fragment } from 'react';
const MyComponent = ()=>{
    return <div data-component="MyComponent" data-source-file="test.jsx">
      <React.Fragment>
        <h1>Using React.Fragment</h1>
        <p>This is inside React.Fragment</p>
      </React.Fragment>
      
      <Fragment>
        <h2>Using Fragment</h2>
        <span>This is inside Fragment</span>
      </Fragment>
      
      <>
        <h3 data-component="MyComponent" data-source-file="test.jsx">Using empty tag syntax</h3>
        <button data-component="MyComponent" data-source-file="test.jsx">This is inside empty tag fragment</button>
      </>
    </div>;
};
const AnotherComponent = ()=>{
    return <>
      <p data-component="AnotherComponent" data-source-file="test.jsx">Root fragment</p>
      <div data-component="AnotherComponent" data-source-file="test.jsx">Content inside root fragment</div>
    </>;
};
const EdgeCasesComponent = ()=>{
    return <div data-component="EdgeCasesComponent" data-source-file="test.jsx">
      { /* Nested fragments */ }
      <Fragment>
        <Fragment>
          <h1>Nested Fragment content</h1>
        </Fragment>
      </Fragment>
      
      { /* Mixed fragment types */ }
      <React.Fragment>
        <>
          <h2>Mixed fragment types</h2>
        </>
      </React.Fragment>
      
      { /* Conditional fragments */ }
      {true ? <Fragment>
          <h3>Conditional fragment</h3>
        </Fragment> : <>
          <h4>Alternative fragment</h4>
        </>}
      
      { /* Fragment with single child */ }
      <Fragment>
        <p>Single child in Fragment</p>
      </Fragment>
      
      { /* Empty tag with single child */ }
      <>
        <p data-component="EdgeCasesComponent" data-source-file="test.jsx">Single child in empty tag</p>
      </>
    </div>;
};
const ConditionalComponent = ()=>{
    return <>
      {true && <div>Conditional content</div>}
      {false || <span>Alternative content</span>}
    </>;
};
export default MyComponent; 