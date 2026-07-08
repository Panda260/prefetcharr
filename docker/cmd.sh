#!/bin/sh

set -e

# Run in compat mode for the old config style
test -z "$PREFETCHARR_CONFIG" -a ! -f /config \
  && exec /prefetcharr \
    --media-server-type "${MEDIA_SERVER_TYPE}" \
    --media-server-url "${MEDIA_SERVER_URL}" \
    --sonarr-url "${SONARR_URL}" \
    --log-dir "${LOG_DIR}" \
    --interval "${INTERVAL:-900}" \
    --remaining-episodes "${REMAINING_EPISODES:-2}" \
    ${USERS:+--users "${USERS}"} \
    ${IGNORE_USERS:+--ignore-users "${IGNORE_USERS}"} \
    ${LIBRARIES:+--libraries "${LIBRARIES}"} \
    --connection-retries 6 

test -f /config || printf "%s\n" "$PREFETCHARR_CONFIG" > /config

exec /prefetcharr --config /config

