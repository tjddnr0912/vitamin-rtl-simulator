cd /private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s582/grounding/mx
for c in $(python3 -c "import json;print(' '.join(json.load(open('spec.json'))['cells']))"); do
  TOOLS="sv vl" /private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s582/grounding/run.sh ${c}_ci.sv > /dev/null 2>&1
  TOOLS="vita sv" /private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s582/grounding/run.sh ${c}_if.sv > /dev/null 2>&1
done
echo done > mx.rc
