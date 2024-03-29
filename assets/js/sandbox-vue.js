// https://konvajs.org/docs/vue/index.html
import VelocityArrow from "./velocity-arrow-vue.js";

// eslint-disable-next-line no-undef
Vue.use(VueKonva);

const PIXEL_HEIGHT = 580;
const HISTORY_SECONDS = 5;
const HISTORY_INCREMENT_SECONDS = 0.1;

const packetHistory = [];

const invoke = window.__TAURI__.invoke;
const listen = window.__TAURI__.event.listen;

export default {
  name: "sandbox",
  components: {
    "velocity-arrow": VelocityArrow,
  },
  template: `
  <div>
    <b-navbar class="navbar">
      <b-navbar-brand>
        <img class="logo" src="imgs/rlbot_logo.png">
        <span class="rlbot-brand" style="flex: 1">RLBot Sandbox</span>
      </b-navbar-brand>
      <b-navbar-nav class="ml-auto">
        <b-button @click="$router.push('/console')" variant="dark" class="ml-2">
          Console
        </b-button>
        <b-button @click="watching = false; $router.replace('/')" variant="dark" class="ml-2">
          Back
        </b-button>
      </b-navbar-nav>
    </b-navbar>
    <b-container fluid>
      <b-row>
        <b-col>
          <b-card class="settings-card">
<!--                        This v-stage thing comes from here: https://konvajs.org/docs/vue/index.html-->
            <v-stage ref="stage" :config="configKonva" class="arena-background">
              <v-layer ref="layer">
                <!-- Order of elements here matters - they will be rendered in that order -->
                <velocity-arrow :object="ball"
                        color="grey"
                        :maxradius="6000"
                        @dragstart="dragging = true"
                        @dragend="handleBallDragEnd">
                </velocity-arrow>
                <velocity-arrow v-for="car in cars"
                        :object="car"
                        :color="car.fill"
                        :maxradius="2300"
                        @dragstart="dragging = true"
                        @dragend="handleCarVelocityDragEnd">
                </velocity-arrow>
                <v-circle :config="ball"
                      @dragstart="handleDragStart"
                      @dragend="handleBallDragEnd"
                      @dragmove="handleBallDragMove">
                </v-circle>
                <v-rect v-for="car in cars" :config="car"
                    @dragstart="handleDragStart"
                    @dragend="handleCarDragEnd"
                    @dragmove="handleCarDragMove">
                </v-rect>
              </v-layer>
            </v-stage>
          </b-card>
        </b-col>
        <b-col>
          <b-card class="settings-card" title="Commands">
            <b-form inline class="mb-2">
              <b-form-checkbox class="mr-4" v-model="watching">
                Watch Game
              </b-form-checkbox>
              <b-form-checkbox v-model="frozen">
                Freeze Game
              </b-form-checkbox>
            </b-form>
            <b-form-radio-group class="mb-2">
              <b-form-radio v-model="gravity" name="gravity-radios" value="normal">Normal gravity</b-form-radio>
              <b-form-radio v-model="gravity" name="gravity-radios" value="zero">Zero gravity</b-form-radio>
            </b-form-radio-group>
            <div class="mb-4">
              <b-button class="md-raised md-alternative" @click="rewind()"  :disabled="!hasPacketHistory">
                Rewind 5 Seconds
              </b-button>
            </div>

            <b-form-group label-cols="3" label-align="right" label="Game Speed" class="form-group-inline">
              <b-form-input v-model="gamespeed" type="number"></b-form-input>
              <b-button @click="setGamespeed()" class="ml-2">Set</b-button>
            </b-form-group>

            <b-form-group label-cols="3" label-align="right" label="Console Command" class="form-group-inline">
              <b-form-input v-model="command"></b-form-input>
              <b-button @click="executeCommand()" class="ml-2">Execute</b-button>
            </b-form-group>

            <p>
              You can drag and drop to move objects around in the game!
            </p>
          </b-card>
        </b-col>
      </b-row>
    </b-container>
  </div>
  `,
  data() {
    return {
      dragging: false,
      watching: false,
      frozen: false,
      gravity: "normal",
      command: null,
      gamespeed: 1,
      gameTickPacket: null,
      previousSecondsElapsed: 0,
      configKonva: {
        width: 410,
        height: PIXEL_HEIGHT,
      },
      ball: {
        x: 100,
        y: 100,
        radius: 10,
        fill: "gray",
        stroke: "black",
        strokeWidth: 2,
        draggable: true,
      },
      cars: [],
      hasPacketHistory: false,
      gtp: listen("gtp", (event) => this.gameTickPacketReceived(event.payload)),
    };
  },
  methods: {
    startWatching: function () {
      invoke("fetch_game_tick_packet_json");
      this.previousSecondsElapsed = 0;
    },
    gameTickPacketReceived: function (result) {
      if (
        !this.dragging &&
        result.game_info.seconds_elapsed > this.previousSecondsElapsed
      ) {
        this.previousSecondsElapsed = result.game_info.seconds_elapsed;

        const ballLoc = this.toCanvasVec(result.game_ball.physics.location);
        const ballVel = result.game_ball.physics.velocity;
        this.ball.x = ballLoc.x;
        this.ball.y = ballLoc.y;
        this.ball.vx = ballVel.x;
        this.ball.vy = ballVel.y;

        this.cars = [];
        for (let i = 0; i < result.game_cars.length; i++) {
          const car = {
            draggable: true,
            fill: result.game_cars[i].team === 0 ? "blue" : "orange",
            width: 16,
            height: 10,
            offset: {
              x: 8,
              y: 5,
            },
            stroke: "black",
            strokeWidth: 2,
            playerIndex: i,
          };

          const phys = result.game_cars[i].physics;
          const carLoc = this.toCanvasVec(phys.location);
          car.x = carLoc.x;
          car.y = carLoc.y;
          car.vx = phys.velocity.x;
          car.vy = phys.velocity.y;
          car.rotation = (phys.rotation.yaw * 180) / Math.PI;
          this.cars.push(car);
        }
      }

      if (packetHistory.length) {
        const tail = packetHistory[packetHistory.length - 1];
        const tailTime = tail.game_info.seconds_elapsed;
        if (
          result.game_info.seconds_elapsed - tailTime >
          HISTORY_INCREMENT_SECONDS
        ) {
          packetHistory.push(result);
          if (
            packetHistory.length >
            HISTORY_SECONDS / HISTORY_INCREMENT_SECONDS
          ) {
            packetHistory.shift();
          }
        }
      } else {
        packetHistory.push(result);
      }
      this.hasPacketHistory = true;

      if (this.watching) {
        // Delay 50 milliseconds to avoid hurting the CPU
        setTimeout(() => invoke("fetch_game_tick_packet_json"), 50);
      }
    },
    toCanvasVec: function (packetVec) {
      // Height without goals: 512 px
      // Total width: 410 px
      return {
        x: packetVec.x / -20 + 205,
        y: packetVec.y / -20 + PIXEL_HEIGHT / 2,
        z: packetVec.z / 20,
      };
    },
    toPacketVec: function (canvasVec) {
      return {
        x: (canvasVec.x - 205) * -20,
        y: (canvasVec.y - PIXEL_HEIGHT / 2) * -20,
        z: canvasVec.z * 20,
      };
    },
    handleDragStart: function () {
      this.dragging = true;
    },
    handleBallDragMove: function (evt) {
      this.ball.x = evt.target.x();
      this.ball.y = evt.target.y();
    },
    handleCarDragMove: function (evt) {
      const index = evt.target.attrs.playerIndex;
      this.cars[index].x = evt.target.x();
      this.cars[index].y = evt.target.y();
    },
    handleBallDragEnd: function () {
      this.dragging = false;
      const packetVec = this.toPacketVec({
        x: this.ball.x,
        y: this.ball.y,
        z: 10,
      });
      invoke("set_state", {
        state: {
          ball: {
            physics: {
              location: packetVec,
              velocity: { x: this.ball.vx, y: this.ball.vy, z: 0 },
            },
          },
        },
      });
    },
    handleCarDragEnd: function (evt) {
      const index = evt.target.attrs.playerIndex;
      this.dragging = false;
      const packetVec = this.toPacketVec({
        x: evt.target.x(),
        y: evt.target.y(),
        z: 1,
      });
      const cars = {};
      cars[index] = { physics: { location: packetVec } };
      invoke("set_state", { state: { cars } });
    },
    handleCarVelocityDragEnd: function (car) {
      this.dragging = false;
      const cars = {};
      cars[car.playerIndex] = {
        physics: {
          location: this.toPacketVec({ x: car.x, y: car.y, z: 1 }),
          velocity: { x: car.vx, y: car.vy, z: 0 },
          rotation: { pitch: 0, yaw: (car.rotation / 180) * Math.PI, roll: 0 },
        },
      };
      invoke("set_state", { state: { cars } });
    },
    executeCommand: function () {
      invoke("set_state", { state: { console_commands: [this.command] } });
    },
    setGamespeed: function () {
      invoke("set_state", {
        state: { game_info: { game_speed: parseFloat(this.gamespeed) } },
      });
    },
    rewind: function () {
      // Rewinds to the earliest point in recorded history. In many cases this will be 5 seconds ago,
      // assuming HISTORY_SECONDS is still configured as 5. However, due to low amount of history data or
      // interruptions in recording, this can vary.

      // The rewinding is not perfect because state setting cannot actually change the clock or change
      // the state of boost pads. Also state setting is a little mushy in general, so consider this to be
      // "best effort".

      if (packetHistory.length > 0) {
        const firstPacket = packetHistory[0];
        const lastPacket = packetHistory[packetHistory.length - 1];

        // Doctor the time on the first packet. It will remain in the packet history array.
        // With the new time, it will be representative of what's about to happen due to state setting.
        // This will prevent any "bad history" from leaking in due to latency before the state setting takes effect.
        firstPacket.game_info.seconds_elapsed =
          lastPacket.game_info.seconds_elapsed;

        // Truncate the packet history
        packetHistory.length = 1;

        const cars = {};
        firstPacket.game_cars.forEach(function (car, index) {
          cars[index] = { physics: car.physics, boost_amount: car.boost };
        });
        invoke("set_state", {
          state: { cars, ball: { physics: firstPacket.game_ball.physics } },
        });
      }
      this.hasPacketHistory = false;
    },
  },
  watch: {
    watching: {
      handler: function (newVal) {
        if (newVal) {
          this.startWatching();
        }
      },
    },
    frozen: {
      handler: function (newVal) {
        invoke("set_state", { state: { game_info: { paused: newVal } } });
      },
    },
    gravity: {
      handler: function (newVal) {
        if (newVal === "zero") {
          invoke("set_state", {
            state: { game_info: { world_gravity_z: -0.000001 } },
          });
        } else {
          invoke("set_state", {
            state: { game_info: { world_gravity_z: -650 } },
          });
        }
      },
    },
  },
};
