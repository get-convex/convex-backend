const http = require('http');

const options = {
  hostname: '35.243.120.253',
  port: 3210,
  path: '/api/query',
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
  }
};

const data = JSON.stringify({
  path: '_system/queryTable',
  args: { tableName: 'users' }
});

const req = http.request(options, (res) => {
  let responseData = '';
  
  res.on('data', (chunk) => {
    responseData += chunk;
  });
  
  res.on('end', () => {
    console.log('Response:', responseData);
  });
});

req.on('error', (error) => {
  console.error('Error:', error);
});

req.write(data);
req.end();