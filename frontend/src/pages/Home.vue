<!-- src/pages/Home.vue-->
 <template>
    <div class="container">
        <h1>Welcome to RustTube</h1>
        <p> Your lightweight, microservice-powered video platform.</p>
        <div class="buttons">
            <router-link to="/login">
                <button>Login</button>
            </router-link>
            <router-link to="/register">
                <button>Register</button>
            </router-link>
        </div>
    </div>
 </template>

 <style scoped>
 .container {
    text-align: center;
    margin-top: 100px;
 }

 h1 {
    font-size: 2.5rem;
    margin-bottom: 1rem;
 }

 p {
    font-size: 1.2rem;
    margin-bottom: 2rem;
 }

 .buttons button {
    margin: 0 10px;
    padding: 10px 20px;
    font-size: 1rem;
 }
 </style>