const http = require('http');

const ADMIN_KEY = 'convex-self-hosted|0183c5c909ade849704ebc5fcac68614ee30dcf9d52f33268bc127e3a4495e18c7b6e2cde7';

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

async function queryUsers() {
  console.log('Querying users table...');
  
  // Try to query all users using system query
  try {
    const result = await makeRequest('/api/query', {
      path: '_internal/query',
      args: {
        tableName: 'users',
        limit: 100
      }
    }, 'POST');
    
    console.log(`Status: ${result.status}`);
    console.log('Users data:', JSON.stringify(result.data, null, 2));
    
    if (result.data && result.data.value) {
      console.log(`\nFound ${result.data.value.length} users`);
      // Save to file
      require('fs').writeFileSync('users-export.json', JSON.stringify(result.data.value, null, 2));
      console.log('Users saved to users-export.json');
    }
    
  } catch (error) {
    console.log('Error:', error.message);
  }

  // Try alternative query format
  console.log('\nTrying alternative query format...');
  try {
    const result = await makeRequest('/api/query', {
      path: 'system:query',
      args: {
        table: 'users'
      }
    }, 'POST');
    
    console.log(`Status: ${result.status}`);
    console.log('Response:', JSON.stringify(result.data, null, 2));
    
  } catch (error) {
    console.log('Error:', error.message);
  }
}

queryUsers();