const http = require('http');
const fs = require('fs');

const DEV_ADMIN_KEY = 'convex-self-hosted|0183c5c909ade849704ebc5fcac68614ee30dcf9d52f33268bc127e3a4495e18c7b6e2cde7';
const PROD_ADMIN_KEY = 'convex-self-hosted|010fdea2b836213103721010e79f51286f1ebfc6841de96d2aa90a7b1aad476ebea291dfaf';

const DEV_URL = 'http://35.243.120.253:3210';
const PROD_URL = 'http://34.84.108.222:3210'; // Production server IP

function makeRequest(baseUrl, adminKey, path, data = null, method = 'GET') {
  return new Promise((resolve, reject) => {
    const url = new URL(baseUrl);
    const options = {
      hostname: url.hostname,
      port: url.port || (url.protocol === 'https:' ? 443 : 80),
      path: path,
      method: method,
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Convex ${adminKey}`,
        'User-Agent': 'Convex-Migration-Tool'
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

async function migrateUsers() {
  console.log('Starting user migration from dev to production...');
  
  try {
    // Step 1: Test connectivity to both environments
    console.log('\n1. Testing connectivity...');
    
    const devShapes = await makeRequest(DEV_URL, DEV_ADMIN_KEY, '/api/shapes2');
    console.log(`Dev environment: ${devShapes.status === 200 ? 'Connected' : 'Failed'}`);
    
    const prodShapes = await makeRequest(PROD_URL, PROD_ADMIN_KEY, '/api/shapes2');
    console.log(`Prod environment: ${prodShapes.status === 200 ? 'Connected' : 'Failed'}`);
    
    if (devShapes.status !== 200 || prodShapes.status !== 200) {
      throw new Error('Failed to connect to one or both environments');
    }
    
    // Step 2: Check if users table exists in both environments
    console.log('\n2. Checking table structures...');
    console.log('Dev tables:', Object.keys(devShapes.data).includes('users') ? 'users table found' : 'users table missing');
    console.log('Prod tables:', Object.keys(prodShapes.data).includes('users') ? 'users table found' : 'users table missing');
    
    // Since we can't directly query the users, we'll need to use manual insertion
    // Based on the dashboard screenshot, I'll create sample records
    const sampleUsers = [
      {
        clerkUserId: "user_2z4hqhPD18Tb",
        department: "unset",
        enrollmentDate: "2024-08-01",
        lastLoginDate: "2024-08-05"
      },
      {
        clerkUserId: "user_2zFzfvpz3RZT",
        department: "unset", 
        enrollmentDate: "2024-08-01",
        lastLoginDate: "2024-08-05"
      }
      // Add more users as needed based on the screenshot data
    ];
    
    console.log('\n3. Sample user migration would insert:', sampleUsers.length, 'users');
    console.log('First user sample:', JSON.stringify(sampleUsers[0], null, 2));
    
    console.log('\nMigration script is ready but needs actual user data extraction.');
    console.log('Next steps:');
    console.log('1. Extract all 28 user records from development');
    console.log('2. Use mutation API to insert into production');
    console.log('3. Verify migration success');
    
  } catch (error) {
    console.error('Migration failed:', error.message);
  }
}

migrateUsers();