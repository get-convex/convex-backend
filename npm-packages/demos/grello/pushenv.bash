while IFS='=' read -r key value; do
  if [[ $key && $value && $key != \#* ]]; then
    npx convex env set "$key" $value
  fi
done < .env.local
