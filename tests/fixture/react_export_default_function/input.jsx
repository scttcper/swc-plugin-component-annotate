import React from 'react';

export default function MyDefaultComponent({ title }) {
  return (
    <Layout>
      <h1>{title}</h1>
      <Button>Click me</Button>
    </Layout>
  );
}
