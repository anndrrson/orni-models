-- Seed platform models (free-tier, featured, pointing to Together.ai)

INSERT INTO users (id, wallet_address, display_name, is_creator)
VALUES ('00000000-0000-0000-0000-000000000001', NULL, 'Orni Platform', TRUE)
ON CONFLICT DO NOTHING;

INSERT INTO models (id, creator_id, slug, name, description, system_prompt, base_model, provider_model_id, status, price_per_query, category, is_featured, is_platform_model, free_queries_per_day)
VALUES
    ('00000000-0000-0000-0000-000000000010',
     '00000000-0000-0000-0000-000000000001',
     'llama-3-8b',
     'Llama 3.1 8B',
     'Meta''s fast and capable open-source model. Great for everyday tasks, coding help, and quick answers.',
     'You are a helpful, friendly AI assistant powered by Llama 3.1 8B. Be concise and accurate.',
     'meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo',
     'meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo',
     'live', 50000, 'Technology',
     TRUE, TRUE, 5),

    ('00000000-0000-0000-0000-000000000011',
     '00000000-0000-0000-0000-000000000001',
     'mistral-7b',
     'Mistral 7B',
     'Mistral AI''s efficient model. Excellent at reasoning, code generation, and multilingual tasks.',
     'You are a helpful AI assistant powered by Mistral 7B. Provide clear, well-structured responses.',
     'mistralai/Mistral-7B-Instruct-v0.3',
     'mistralai/Mistral-7B-Instruct-v0.3',
     'live', 50000, 'Technology',
     TRUE, TRUE, 5),

    ('00000000-0000-0000-0000-000000000012',
     '00000000-0000-0000-0000-000000000001',
     'llama-3-70b',
     'Llama 3.1 70B',
     'Meta''s most capable open model. Best for complex reasoning, detailed analysis, and creative writing.',
     'You are a helpful, knowledgeable AI assistant powered by Llama 3.1 70B. Provide thorough, accurate responses.',
     'meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo',
     'meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo',
     'live', 200000, 'Technology',
     TRUE, TRUE, 3),

    ('00000000-0000-0000-0000-000000000013',
     '00000000-0000-0000-0000-000000000001',
     'code-llama-34b',
     'Code Llama 34B',
     'Specialized coding assistant. Write, debug, and explain code in any language.',
     'You are a coding assistant powered by Code Llama 34B. Help users write clean, efficient code. Always include explanations with your code.',
     'codellama/CodeLlama-34b-Instruct-hf',
     'codellama/CodeLlama-34b-Instruct-hf',
     'live', 100000, 'Technology',
     TRUE, TRUE, 5)
ON CONFLICT DO NOTHING;
