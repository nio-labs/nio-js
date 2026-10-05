import { get } from 'nio.js'
import { z } from 'https://unpkg.com/zod@3.23.8/lib/index.mjs'
get('/', () => z.string().parse('Hello World'))
