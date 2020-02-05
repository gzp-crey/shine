copy ..\secret.config.json binary\
docker build -t shine --build-arg GIT_ACCESS_TOKEN=%1 . 
docker_prune.bat