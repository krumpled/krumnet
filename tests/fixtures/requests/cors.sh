curl 'http://krumnet.local.krumpled.com:8080/auth/identify' \
  -X 'OPTIONS' \
  -H 'Connection: keep-alive' \
  -H 'Pragma: no-cache' \
  -H 'Cache-Control: no-cache' \
  -H 'Access-Control-Request-Method: GET' \
  -H 'Origin: http://krumi.local.krumpled.com:8081' \
  -H 'User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_14_6) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/81.0.4044.138 Safari/537.36' \
  -H 'Access-Control-Request-Headers: authorization' \
  -H 'Accept: */*' \
  -H 'Referer: http://krumi.local.krumpled.com:8081/' \
  -H 'Accept-Language: en-US,en;q=0.9,la;q=0.8' \
  --compressed \
  --insecure \
  --verbose
