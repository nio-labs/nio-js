import { get, asset, redirect } from 'nio.js'
get('/', asset('hello.txt'))
get('/docs', redirect('https://example.com/docs'))
