import React from 'react';
const SentryComponent = ()=>{
    return <div data-sentry-component="SentryComponent" data-sentry-source-file="test.jsx">
      <h1>Sentry Tracking</h1>
      <CustomButton data-sentry-element="CustomButton" data-sentry-source-file="test.jsx">Click me</CustomButton>
    </div>;
};
export default SentryComponent; 