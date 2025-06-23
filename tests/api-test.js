import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
    vus: 1,        // Single virtual user
    iterations: 1, // Run only once
};

const BASE_URL = 'http://localhost:3000';

export default function () {
    console.log('🚀 Starting RAG API comprehensive test suite...\n');
    
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

    if (healthCheckPassed) {
        console.log('✓ Health check passed - service is running correctly\n');
    } else {
        console.log('✗ Health check failed');
        console.log('Response status:', healthResponse.status);
        console.log('Response body:', healthResponse.body);
        return; // Stop testing if health check fails
    }

    // Test 2: Create embedding task
    console.log('2️⃣ Testing create embedding task...');
    const createTaskPayload = {
        file_name: 'test-document.txt',
        file_content: 'aGVsbG8='  // "hello" in base64
    };

    const createResponse = http.post(
        `${BASE_URL}/api/v1/embedding-tasks`,
        JSON.stringify(createTaskPayload),
        {
            headers: {
                'Content-Type': 'application/json',
            },
        }
    );

    let taskId = null;
    const createCheckPassed = check(createResponse, {
        'create task status is 201': (r) => r.status === 201,
        'create task returns valid response': (r) => {
            try {
                const body = JSON.parse(r.body);
                taskId = body.id;
                return body.file_name === 'test-document.txt' && 
                       body.status === 'pending' && 
                       body.id > 0;
            } catch (e) {
                console.log('Failed to parse create task response:', r.body);
                return false;
            }
        },
    });

    if (createCheckPassed) {
        console.log(`✓ Task created successfully with ID: ${taskId}\n`);
    } else {
        console.log('✗ Create task failed');
        console.log('Response status:', createResponse.status);
        console.log('Response body:', createResponse.body);
        return;
    }

    // Test 3: List embedding tasks
    console.log('3️⃣ Testing list embedding tasks...');
    const listResponse = http.get(`${BASE_URL}/api/v1/embedding-tasks`);

    const listCheckPassed = check(listResponse, {
        'list tasks status is 200': (r) => r.status === 200,
        'list tasks returns array': (r) => {
            try {
                const body = JSON.parse(r.body);
                return Array.isArray(body) && body.length > 0;
            } catch (e) {
                console.log('Failed to parse list tasks response:', r.body);
                return false;
            }
        },
    });

    if (listCheckPassed) {
        console.log('✓ Task list retrieved successfully\n');
    } else {
        console.log('✗ List tasks failed');
        console.log('Response status:', listResponse.status);
        console.log('Response body:', listResponse.body);
    }

    // Test 4: Get specific task
    console.log('4️⃣ Testing get specific task...');
    const getResponse = http.get(`${BASE_URL}/api/v1/embedding-tasks/${taskId}`);

    const getCheckPassed = check(getResponse, {
        'get task status is 200': (r) => r.status === 200,
        'get task returns correct data': (r) => {
            try {
                const body = JSON.parse(r.body);
                return body.id === taskId && 
                       body.file_name === 'test-document.txt' && 
                       body.status === 'pending';
            } catch (e) {
                console.log('Failed to parse get task response:', r.body);
                return false;
            }
        },
    });

    if (getCheckPassed) {
        console.log(`✓ Task ${taskId} retrieved successfully\n`);
    } else {
        console.log('✗ Get task failed');
        console.log('Response status:', getResponse.status);
        console.log('Response body:', getResponse.body);
    }

    // Test 5: Update task status
    console.log('5️⃣ Testing update task status...');
    const updatePayload = {
        status: 'completed',
        embedding_count: 42
    };

    const updateResponse = http.put(
        `${BASE_URL}/api/v1/embedding-tasks/${taskId}`,
        JSON.stringify(updatePayload),
        {
            headers: {
                'Content-Type': 'application/json',
            },
        }
    );

    const updateCheckPassed = check(updateResponse, {
        'update task status is 200': (r) => r.status === 200,
        'update task returns updated data': (r) => {
            try {
                const body = JSON.parse(r.body);
                return body.id === taskId && 
                       body.status === 'completed' && 
                       body.embedding_count === 42 &&
                       body.completed_at !== null;
            } catch (e) {
                console.log('Failed to parse update task response:', r.body);
                return false;
            }
        },
    });

    if (updateCheckPassed) {
        console.log(`✓ Task ${taskId} updated successfully to completed status\n`);
    } else {
        console.log('✗ Update task failed');
        console.log('Response status:', updateResponse.status);
        console.log('Response body:', updateResponse.body);
    }

    // Test 6: List tasks with status filter
    console.log('6️⃣ Testing list tasks with status filter...');
    const filterResponse = http.get(`${BASE_URL}/api/v1/embedding-tasks?status=completed`);

    const filterCheckPassed = check(filterResponse, {
        'filter tasks status is 200': (r) => r.status === 200,
        'filter tasks returns completed tasks': (r) => {
            try {
                const body = JSON.parse(r.body);
                return Array.isArray(body) && 
                       body.length > 0 && 
                       body.every(task => task.status === 'completed');
            } catch (e) {
                console.log('Failed to parse filter tasks response:', r.body);
                return false;
            }
        },
    });

    if (filterCheckPassed) {
        console.log('✓ Task filtering by status works correctly\n');
    } else {
        console.log('✗ Filter tasks failed');
        console.log('Response status:', filterResponse.status);
        console.log('Response body:', filterResponse.body);
    }

    // Test 7: Delete task
    console.log('7️⃣ Testing delete task...');
    const deleteResponse = http.del(`${BASE_URL}/api/v1/embedding-tasks/${taskId}`);

    const deleteCheckPassed = check(deleteResponse, {
        'delete task status is 204': (r) => r.status === 204,
    });

    if (deleteCheckPassed) {
        console.log(`✓ Task ${taskId} deleted successfully\n`);
    } else {
        console.log('✗ Delete task failed');
        console.log('Response status:', deleteResponse.status);
        console.log('Response body:', deleteResponse.body);
    }

    // Test 8: Verify task is deleted
    console.log('8️⃣ Testing verify task deletion...');
    const verifyDeleteResponse = http.get(`${BASE_URL}/api/v1/embedding-tasks/${taskId}`);

    const verifyDeleteCheckPassed = check(verifyDeleteResponse, {
        'verify delete status is 404': (r) => r.status === 404,
        'verify delete returns error': (r) => {
            try {
                const body = JSON.parse(r.body);
                return body.error === 'Task not found';
            } catch (e) {
                console.log('Failed to parse verify delete response:', r.body);
                return false;
            }
        },
    });

    if (verifyDeleteCheckPassed) {
        console.log(`✓ Task ${taskId} confirmed deleted - returns 404 as expected\n`);
    } else {
        console.log('✗ Verify delete failed');
        console.log('Response status:', verifyDeleteResponse.status);
        console.log('Response body:', verifyDeleteResponse.body);
    }

    console.log('🎉 RAG API comprehensive test suite completed!');
    console.log('📊 Test summary: All embedding task CRUD operations tested');
} 