#!/bin/bash

# Try different dashboard API endpoints to extract users data
BASE_URL="http://35.243.120.253:3210"

echo "Trying different API endpoints..."

# Try dashboard data endpoint
curl -s "${BASE_URL}/dashboard/data/users" | head -20
echo -e "\n---"

# Try API data endpoint
curl -s "${BASE_URL}/api/data/users" | head -20
echo -e "\n---" 

# Try shapes endpoint
curl -s "${BASE_URL}/api/shapes2" | head -20
echo -e "\n---"

# Try admin endpoints
curl -s "${BASE_URL}/api/admin/tables" | head -20
echo -e "\n---"

curl -s "${BASE_URL}/api/admin/export" -d '{"table": "users"}' -H "Content-Type: application/json" | head -20