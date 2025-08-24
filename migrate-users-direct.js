const http = require('http');

const DEV_ADMIN_KEY = 'convex-self-hosted|0183c5c909ade849704ebc5fcac68614ee30dcf9d52f33268bc127e3a4495e18c7b6e2cde7';
const DEV_BASE_URL = 'http://35.243.120.253:3210';

// Production environment details (we'll need the production admin key)
const PROD_BASE_URL = 'http://35.243.120.253:3210'; // This will need to be updated with prod URL

// The user data I saw in the dashboard screenshot
const SAMPLE_USERS = [
  {
    _id: "q5792b5ypvk97z0jcr...",
    clerkUserId: "user_2z4hqhPD18Tb...",
    department: "unset",
    enrollmentDate: "2024-xx-xx",
    // Add other fields as needed
  }
  // Add all 28 users here
];

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

async function migrateUsers() {
  console.log('Starting user migration from dev to prod...');
  
  // For now, let's just demonstrate the concept
  // In a real migration, we would:
  // 1. Extract all users from dev environment
  // 2. Connect to production environment
  // 3. Insert users into production
  
  console.log('This is a template migration script.');
  console.log('To complete the migration, we need:');
  console.log('1. Production environment admin key');
  console.log('2. Production environment URL');
  console.log('3. Method to extract all 28 users from development');
}

migrateUsers();