const http = require('http');

const ADMIN_KEY = 'convex-self-hosted|0183c5c909ade849704ebc5fcac68614ee30dcf9d52f33268bc127e3a4495e18c7b6e2cde7';
const BASE_URL = 'http://35.243.120.253:3210';

function makeRequest(path, data = null, method = 'GET') {
  return new Promise((resolve, reject) => {
    const options = {
      hostname: '35.243.120.253',
      port: 3210,
      path: path,
      method: method,
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Convex ${ADMIN_KEY}`,
        'User-Agent': 'Convex-Dashboard'
      }
    };

    const req = http.request(options, (res) => {
      let responseData = '';
      
      res.on('data', (chunk) => {
        responseData += chunk;
      });
      
      res.on('end', () => {
        try {
          const parsed = JSON.parse(responseData);
          resolve({ status: res.statusCode, data: parsed });
        } catch (e) {
          resolve({ status: res.statusCode, data: responseData });
        }
      });
    });

    req.on('error', (error) => {
      reject(error);
    });

    if (data) {
      req.write(JSON.stringify(data));
    }
    req.end();
  });
}

async function exportUsers() {
  console.log('Trying to export users data...');
  
  // Try different API endpoints
  const endpoints = [
    '/api/export',
    '/api/snapshot_export', 
    '/api/admin/export',
    '/api/admin/snapshot_export',
    '/dashboard/export',
    '/api/query'
  ];

  for (const endpoint of endpoints) {
    console.log(`\nTrying ${endpoint}...`);
    try {
      const result = await makeRequest(endpoint, {
        tables: ['users'],
        format: 'jsonl'
      }, 'POST');
      console.log(`Status: ${result.status}`);
      console.log('Response:', JSON.stringify(result.data, null, 2));
    } catch (error) {
      console.log('Error:', error.message);
    }
  }

  // Try getting shapes/schema info
  console.log('\nTrying shapes API...');
  try {
    const result = await makeRequest('/api/shapes2');
    console.log(`Status: ${result.status}`);
    if (result.data && typeof result.data === 'object') {
      console.log('Shapes response keys:', Object.keys(result.data));
    } else {
      console.log('Response:', result.data);
    }
  } catch (error) {
    console.log('Error:', error.message);
  }
}

exportUsers();