import http from 'k6/http';
import { check, sleep } from 'k6';
import encoding from 'k6/encoding';

const username = 'TODO';
const password = 'password';

export const options = {
  vus: 5, // Key for Smoke test. Keep it at 2, 3, max 5 VUs
  duration: '30s', // This can be shorter or just a few iterations
};

export default () => {
  const credentials = `${username}:${password}`;  

  const encodedCredentials = encoding.b64encode(credentials);
  const options = {
    headers: {
      Authorization: `Basic ${encodedCredentials}`,
    },
  };

  const res = http.get('http://localhost:3030/simple/', options);

  check(res, {
    'status is 200': (r) => r.status === 200,
  });
  
  // sleep(1);
  // MORE STEPS
  // Here you can have more steps or complex script
  // Step1
  // Step2
  // etc.
};
