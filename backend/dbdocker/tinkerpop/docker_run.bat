echo off

set DIR=%~dp0
set NAME=con_shine_db_tinkerpop

docker container inspect %NAME% > NUL

if %ERRORLEVEL% EQU 0 ( 
    echo Connecting to a running container: %NAME%
    docker start -ai %NAME% 
) else ( 
    echo Starting a new container: %NAME%
    docker run -ti -p 8182:8182 -v %DIR%data:/data/graph_files --name %NAME% shine_db_tinkerpop %* 
)

    
