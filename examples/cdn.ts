import { get } from 'nio.js'
import { z } from 'https://esm.sh/zod@3.23.8?target=es2022'
get('/', () => z.string().parse('Hello World'))
