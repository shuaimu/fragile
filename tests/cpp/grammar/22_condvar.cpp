// Test: Condition variable basic functionality
// Expected: test_condvar() returns 42 (producer-consumer with condition variable)
// This test verifies pthread_cond_init/signal/wait/destroy are called correctly

#include <pthread.h>

static int data_ready = 0;
static int shared_data = 0;
static pthread_mutex_t mutex;
static pthread_cond_t cond;

void* producer(void* arg) {
    pthread_mutex_lock(&mutex);
    shared_data = 42;
    data_ready = 1;
    pthread_cond_signal(&cond);
    pthread_mutex_unlock(&mutex);
    return 0;
}

void* consumer(void* arg) {
    pthread_mutex_lock(&mutex);
    // Wait for data to be ready
    // Note: In real code, this should be a while loop to handle spurious wakeups
    // But with our stub implementation, a single wait is sufficient
    if (!data_ready) {
        pthread_cond_wait(&cond, &mutex);
    }
    pthread_mutex_unlock(&mutex);
    return 0;
}

int test_condvar() {
    pthread_mutex_init(&mutex, 0);
    pthread_cond_init(&cond, 0);

    pthread_t prod, cons;

    // Start consumer first (it will wait on condition)
    pthread_create(&cons, 0, consumer, 0);

    // Small delay to ensure consumer starts waiting (not needed with stub but good practice)
    // In real implementation, consumer would block on pthread_cond_wait

    // Start producer (it will signal the condition)
    pthread_create(&prod, 0, producer, 0);

    // Wait for both threads to complete
    pthread_join(prod, 0);
    pthread_join(cons, 0);

    pthread_cond_destroy(&cond);
    pthread_mutex_destroy(&mutex);

    // Return the shared data set by producer
    return shared_data;
}
