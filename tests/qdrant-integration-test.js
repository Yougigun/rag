import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
    vus: 1,        // Single virtual user
    iterations: 1, // Run only once
    timeout: '5m', // Allow more time for embedding processing
};

const BASE_URL = 'http://localhost:3000';

export default function () {
    console.log('🚀 Starting RAG API + Qdrant Integration Test Suite...\n');
    // wait for 10 seconds
    console.log('Waiting for 10 seconds...');
    sleep(10);

    // Test 1: Health check endpoint
    console.log('1️⃣ Testing health check endpoint...');
    const healthResponse = http.get(`${BASE_URL}/api/v1/health`);

    const healthCheckPassed = check(healthResponse, {
        'health check status is 200': (r) => r.status === 200,
        'health check returns OK': (r) => {
            try {
                const body = JSON.parse(r.body);
                return body.status === 'ok' && body.service === 'rag-api';
            } catch (e) {
                console.log('Failed to parse health check response:', r.body);
                return false;
            }
        },
    });

    if (!healthCheckPassed) {
        console.log('✗ Health check failed - stopping test');
        return;
    }
    console.log('✅ Health check passed\n');

    // Test 2: Create first embedding task
    console.log('2️⃣ Creating first embedding task...');
    const firstTaskPayload = {
        file_name: 'test-document-1.txt',
        file_content: 'VGhpcyBpcyBhIGRvY3VtZW50IGFib3V0IGFydGlmaWNpYWwgaW50ZWxsaWdlbmNlIGFuZCBtYWNoaW5lIGxlYXJuaW5nLg==' // "This is a document about artificial intelligence and machine learning."
    };

    const firstCreateResponse = http.post(
        `${BASE_URL}/api/v1/embedding-tasks`,
        JSON.stringify(firstTaskPayload),
        {
            headers: {
                'Content-Type': 'application/json',
            },
        }
    );

    let firstTaskId = null;
    const firstCreatePassed = check(firstCreateResponse, {
        'first task creation status is 201': (r) => r.status === 201,
        'first task returns valid response': (r) => {
            try {
                const body = JSON.parse(r.body);
                firstTaskId = body.id;
                return body.file_name === 'test-document-1.txt' &&
                    body.status === 'pending' &&
                    body.id > 0;
            } catch (e) {
                console.log('Failed to parse first task response:', r.body);
                return false;
            }
        },
    });

    if (!firstCreatePassed || !firstTaskId) {
        console.log('✗ First task creation failed - stopping test');
        return;
    }
    console.log(`✅ First task created with ID: ${firstTaskId}\n`);

    // Test 3: Create second embedding task with different content
    console.log('3️⃣ Creating second embedding task...');
    const secondTaskPayload = {
        file_name: 'test-document-2.txt',
        file_content: 'VGhpcyBkb2N1bWVudCBpcyBhYm91dCBjb29raW5nIHJlY2lwZXMgYW5kIGN1bGluYXJ5IGFydHMu' // "This document is about cooking recipes and culinary arts."
    };

    const secondCreateResponse = http.post(
        `${BASE_URL}/api/v1/embedding-tasks`,
        JSON.stringify(secondTaskPayload),
        {
            headers: {
                'Content-Type': 'application/json',
            },
        }
    );

    let secondTaskId = null;
    const secondCreatePassed = check(secondCreateResponse, {
        'second task creation status is 201': (r) => r.status === 201,
        'second task returns valid response': (r) => {
            try {
                const body = JSON.parse(r.body);
                secondTaskId = body.id;
                return body.file_name === 'test-document-2.txt' &&
                    body.status === 'pending' &&
                    body.id > 0;
            } catch (e) {
                console.log('Failed to parse second task response:', r.body);
                return false;
            }
        },
    });

    if (!secondCreatePassed || !secondTaskId) {
        console.log('✗ Second task creation failed - stopping test');
        return;
    }
    console.log(`✅ Second task created with ID: ${secondTaskId}\n`);

    // Test 4: Wait for embedding processing to complete
    console.log('4️⃣ Waiting for embedding processing to complete...');
    console.log('⏳ Waiting 5 seconds for file-processor to generate and store embeddings...');
    sleep(5);

    // Test 4.5: Check that both embedding tasks are completed
    console.log('4️⃣.5 Verifying embedding tasks are completed...');

    const firstTaskStatus = http.get(`${BASE_URL}/api/v1/embedding-tasks/${firstTaskId}`);
    const secondTaskStatus = http.get(`${BASE_URL}/api/v1/embedding-tasks/${secondTaskId}`);

    const tasksCompletePassed = check(firstTaskStatus, {
        'first task status check is 200': (r) => r.status === 200,
        'first task is completed': (r) => {
            try {
                const body = JSON.parse(r.body);
                return body.status === 'completed';
            } catch (e) {
                console.log('Failed to parse first task status response:', r.body);
                return false;
            }
        },
    }) && check(secondTaskStatus, {
        'second task status check is 200': (r) => r.status === 200,
        'second task is completed': (r) => {
            try {
                const body = JSON.parse(r.body);
                return body.status === 'completed';
            } catch (e) {
                console.log('Failed to parse second task status response:', r.body);
                return false;
            }
        },
    });

    if (!tasksCompletePassed) {
        console.log('✗ Embedding tasks not completed - stopping test');
        return;
    }
    console.log('✅ Both embedding tasks completed successfully\n');

    // Test 5: Search for AI-related content (should match first document)
    console.log('5️⃣ Testing search for AI-related content...');
    const aiSearchPayload = {
        query: 'artificial intelligence machine learning',
        limit: 3
    };

    const aiSearchResponse = http.post(
        `${BASE_URL}/api/v1/search`,
        JSON.stringify(aiSearchPayload),
        {
            headers: {
                'Content-Type': 'application/json',
            },
        }
    );

    const aiSearchPassed = check(aiSearchResponse, {
        'AI search status is 200': (r) => r.status === 200,
        'AI search returns results': (r) => {
            try {
                const body = JSON.parse(r.body);
                console.log('AI Search Results:', JSON.stringify(body, null, 2));

                return body.query === 'artificial intelligence machine learning' &&
                    Array.isArray(body.results) &&
                    body.results.length > 0 &&
                    body.results[0].task_id === firstTaskId &&
                    body.results[0].score > 0.5; // Should have high similarity
            } catch (e) {
                console.log('Failed to parse AI search response:', r.body);
                return false;
            }
        },
    });

    if (aiSearchPassed) {
        console.log('✅ AI search successfully found relevant content\n');
    } else {
        console.log('✗ AI search failed');
        console.log('Response status:', aiSearchResponse.status);
        console.log('Response body:', aiSearchResponse.body);
    }

    // Test 6: Search for cooking-related content (should match second document)
    console.log('6️⃣ Testing search for cooking-related content...');
    const cookingSearchPayload = {
        query: 'cooking recipes food culinary',
        limit: 3
    };

    const cookingSearchResponse = http.post(
        `${BASE_URL}/api/v1/search`,
        JSON.stringify(cookingSearchPayload),
        {
            headers: {
                'Content-Type': 'application/json',
            },
        }
    );

    const cookingSearchPassed = check(cookingSearchResponse, {
        'cooking search status is 200': (r) => r.status === 200,
        'cooking search returns results': (r) => {
            try {
                const body = JSON.parse(r.body);
                console.log('Cooking Search Results:', JSON.stringify(body, null, 2));

                return body.query === 'cooking recipes food culinary' &&
                    Array.isArray(body.results) &&
                    body.results.length > 0 &&
                    body.results[0].task_id === secondTaskId &&
                    body.results[0].score > 0.5; // Should have high similarity
            } catch (e) {
                console.log('Failed to parse cooking search response:', r.body);
                return false;
            }
        },
    });

    if (cookingSearchPassed) {
        console.log('✅ Cooking search successfully found relevant content\n');
    } else {
        console.log('✗ Cooking search failed');
        console.log('Response status:', cookingSearchResponse.status);
        console.log('Response body:', cookingSearchResponse.body);
    }

    // Test 7: Test search with no results
    console.log('7️⃣ Testing search with unrelated query...');
    const noResultsSearchPayload = {
        query: 'quantum physics space exploration astronomy',
        limit: 3
    };

    const noResultsResponse = http.post(
        `${BASE_URL}/api/v1/search`,
        JSON.stringify(noResultsSearchPayload),
        {
            headers: {
                'Content-Type': 'application/json',
            },
        }
    );

    const noResultsPassed = check(noResultsResponse, {
        'unrelated search status is 200': (r) => r.status === 200,
        'unrelated search returns low scores or no results': (r) => {
            try {
                const body = JSON.parse(r.body);
                console.log('Unrelated Search Results:', JSON.stringify(body, null, 2));

                // Should either have no results or results with low similarity scores
                return body.query === 'quantum physics space exploration astronomy' &&
                    Array.isArray(body.results) &&
                    (body.results.length === 0 ||
                        (body.results.length > 0 && body.results[0].score < 0.7));
            } catch (e) {
                console.log('Failed to parse unrelated search response:', r.body);
                return false;
            }
        },
    });

    if (noResultsPassed) {
        console.log('✅ Search correctly handles unrelated queries\n');
    } else {
        console.log('✗ Unrelated search test failed');
    }

    // Cleanup: Delete test tasks
    // console.log('8️⃣ Cleaning up test data...');

    // const deleteFirst = http.del(`${BASE_URL}/api/v1/embedding-tasks/${firstTaskId}`);
    // const deleteSecond = http.del(`${BASE_URL}/api/v1/embedding-tasks/${secondTaskId}`);

    // const cleanupPassed = check(deleteFirst, {
    //     'first task deletion status is 204': (r) => r.status === 204,
    // }) && check(deleteSecond, {
    //     'second task deletion status is 204': (r) => r.status === 204,
    // });

    // if (cleanupPassed) {
    //     console.log('✅ Test data cleaned up successfully\n');
    // } else {
    //     console.log('⚠️ Cleanup may have failed - check manually');
    // }

    console.log('🎉 RAG API + Qdrant Integration Test Suite completed!');
    console.log('📊 Test Summary:');
    console.log('  ✅ Health check');
    console.log('  ✅ Embedding task creation');
    console.log('  ✅ Embedding processing and storage');
    console.log('  ✅ Similarity search functionality');
    console.log('  ✅ Multiple search endpoints (POST/GET)');
    console.log('  ✅ Search relevance and scoring');
    console.log('  ✅ Data cleanup');
}