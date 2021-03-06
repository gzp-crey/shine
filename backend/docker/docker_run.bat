echo off

set DIR=%~dp0
set NAME=con_shine

docker container inspect %NAME% > NUL

if %ERRORLEVEL% EQU 0 ( 
    echo Connecting to a running container: %NAME%
    docker start -ai %NAME% 
) else ( 
    echo Starting a new container: %NAME%
    docker run -ti -p 12346:12345 -v %DIR%..:/webapp/source_dev --name %NAME% shine %* 
)
    
    
