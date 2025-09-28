# Manual test script for MCP server with Content-Length headers

# Test 1: Initialize request
Write-Host "Test 1: Sending initialize request..." -ForegroundColor Yellow
$request = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
$content_length = [System.Text.Encoding]::UTF8.GetByteCount($request)
$message = "Content-Length: $content_length`r`n`r`n$request"

$message | Out-File -FilePath test_input.txt -Encoding UTF8 -NoNewline

Write-Host "Starting server and sending request..."
Get-Content test_input.txt | cargo run 2>$null | Out-File test_output.txt -Encoding UTF8

Write-Host "Response:" -ForegroundColor Green
Get-Content test_output.txt

# Clean up
Remove-Item test_input.txt -ErrorAction SilentlyContinue
Remove-Item test_output.txt -ErrorAction SilentlyContinue