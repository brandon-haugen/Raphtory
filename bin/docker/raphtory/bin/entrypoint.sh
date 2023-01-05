#!/bin/bash
#set -x
set -euf -o pipefail

RAPHTORY_JAVA_RUN_ARGS="${RAPHTORY_JAVA_RUN_ARGS:-}"

#CORE_CP="$(raphtory-classpath)"
CORE_CP="/opt/venv/lib/python3.10/site-packages/pyraphtory/lib/*:/opt/venv/lib/python3.10/site-packages/pyraphtory/lib/runtime/*"
set -x
java -cp "$CORE_CP:/raphtory/jars/*" $RAPHTORY_JAVA_RUN_ARGS $RAPHTORY_JAVA_RUN_CLASS
