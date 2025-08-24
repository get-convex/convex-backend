#!/bin/bash

BASE_URL="http://35.243.120.253:3210"
ADMIN_KEY="convex-self-hosted|0183c5c909ade849704ebc5fcac68614ee30dcf9d52f33268bc127e3a4495e18c7b6e2cde7"

echo "Exporting users data with admin key..."

# Try shapes API with admin key
curl -s "${BASE_URL}/api/shapes2" \
  -H "Authorization: Bearer ${ADMIN_KEY}" \
  -H "Content-Type: application/json" > shapes_response.json

echo "Shapes response saved to shapes_response.json"
cat shapes_response.json | jq . | head -50

echo -e "\n---\nTrying export API..."

# Try export API
curl -s "${BASE_URL}/api/snapshot_export" \
  -H "Authorization: Bearer ${ADMIN_KEY}" \
  -H "Content-Type: application/json" \
  -d '{"tables": ["users"]}' > export_response.json

echo "Export response saved to export_response.json"
cat export_response.json | head -20