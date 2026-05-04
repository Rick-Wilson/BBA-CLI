Set WshShell = CreateObject("WScript.Shell") 
WshShell.CurrentDirectory = "G:\BBA-CLI\bba-server\bin\Debug\net8.0-windows" 
WshShell.Run """G:\BBA-CLI\bba-server\bin\Debug\net8.0-windows\bba-server.exe"" --urls=http://0.0.0.0:5000", 0, False 
