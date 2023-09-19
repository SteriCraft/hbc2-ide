cppFilesLineNumber=$(wc -l HBC-2_IDE/*.cpp | tail -1)
cppFilesLineNumber=$(echo $cppFilesLineNumber|cut -d' ' -f1)

hFilesLineNumber=$(wc -l HBC-2_IDE/*.h | tail -1)
hFilesLineNumber=$(echo $hFilesLineNumber|cut -d' ' -f1)

total=$((cppFilesLineNumber + hFilesLineNumber))
echo Lines of code: $total

debugExecSize=$(du -s HBC-2_IDE/build-HBC-2_IDE-Desktop-Debug/HBC-2_IDE)
debugExecSize=$(echo $debugExecSize|cut -d' ' -f1)
echo Debug executable size: $debugExecSize kB

releaseExec=$(du -s HBC-2_IDE/build-HBC-2_IDE-Desktop-Release/HBC-2_IDE)
releaseExec=$(echo $releaseExec|cut -d' ' -f1)
echo Release executable size: $releaseExec kB