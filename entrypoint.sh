#!/bin/bash
set -euo pipefail

# entrypoint: wait for DATABASE_URL target to be reachable before execing command
if [ -n "${DATABASE_URL:-}" ]; then
  PYTHON_EXEC="${PYO3_PYTHON:-/opt/conda/bin/python}"
  # small python snippet to wait for tcp port
  "$PYTHON_EXEC" - <<'PY'
import os,sys,time,socket,urllib.parse
url=os.environ.get('DATABASE_URL')
if not url:
    sys.exit(0)
u=urllib.parse.urlparse(url)
host=u.hostname or 'localhost'
port=u.port or 5432
timeout=60
end=time.time()+timeout
while time.time()<end:
    try:
        s=socket.create_connection((host,port),2)
        s.close()
        sys.exit(0)
    except Exception:
        time.sleep(1)
sys.exit(1)
PY
fi

# exec provided command
exec "$@"
