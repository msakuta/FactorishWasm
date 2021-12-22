<script>
import CloseButton from "./CloseButton.vue";
import ItemIcon from './ItemIcon.vue';
import { nextTick, ref } from "vue";

export default {
  components: {
    CloseButton,
    ItemIcon,
  },

  props: {
    dragWindowMouseDown: Function,
    onShowNewGame: Function,
    onShowViewSettings: Function,
    serializer: Function,
    deserializer: Function,
    bringToTop: Function,
  },

  setup(props, context) {
    const {
      serializer,
      deserializer,
      dragWindowMouseDown,
    } = props;

    const visible = ref(false);
    const saveText = ref(null);
    const loadFile = ref(null);

    return {
      visible,
      saveText,
      loadFile,
      left: ref(0),
      top: ref(0),

      zIndex: ref(0),
      dragWindowMouseDown,

      onSave() {
        var textFileAsBlob = new Blob([serializer()], {
            type: 'text/json'
        });
        var fileNameToSaveAs = "save.json";
    
        var downloadLink = document.createElement("a");
        downloadLink.download = fileNameToSaveAs;
        downloadLink.innerHTML = "Download File";
        let appended = false;
        if (window.webkitURL != null) {
            downloadLink.href = window.webkitURL.createObjectURL(textFileAsBlob);
        }
        else {
            downloadLink.href = window.URL.createObjectURL(textFileAsBlob);
            downloadLink.style.display = "none";
            document.body.appendChild(downloadLink);
            appended = true;
        }
        downloadLink.click();
        if(appended) {
            document.body.removeChild(downloadLink);
        }
      },

      onLoad(event) {
        loadFile.value.click();
      },

      onLoadFileChange(event) {
        const reader = new FileReader();
        reader.onload = (event) => deserializer(event.target.result);
        reader.readAsText(event.target.files[0]);
        visible.value = false;
      },

      inventoryDragStart: null,
      close(){
        console.log("Handling click: " + visible.value);
        visible.value = !visible.value;
      },
    };
  },

  methods: {
    dragWindow(evt){
      this.dragWindowMouseDown(
        evt,
        this.$refs.root,
        this,
        this.inventoryDragStart,
        (x, y) => {
          this.left = x;
          this.top = y;
        }
      )
    },

    // Place a window element at the center, for Vue component.
    placeCenter() {
      // Defer one tick to allow DOM element to be created
      nextTick(() => {
        if (!this.$refs.root) return;
        var elemRect = this.$refs.root.getBoundingClientRect();
        var bodyRect = document.body.getBoundingClientRect();
        this.left = (bodyRect.width - elemRect.width) / 2;
        this.top = (bodyRect.height - elemRect.height) / 2;
      });
    },
  },
};
</script>

<template>
  <div v-if="visible" ref="root"
    :class="['noselect', 'windowFrame']"
    :style="{left: `${left}px`, top: `${top}px`, zIndex}"
    @click="bringToTop"
  >
    <div ref="recipeTitle" class="inventoryTitle" @mousedown="dragWindow">Main Menu</div>
    <close-button @click="close"></close-button>
    <div :style="{
          textAlign: 'center',
          margin: '16px',
        }">
      <h1>FactorishWasm</h1>
      <p style="font-size: 80%; font-style: italic">- Factorio-style base building game with WebAssembly -</p>
      <button @click="onShowNewGame" class="largeButton">New game</button>
      <button @click="onShowViewSettings" class="largeButton">View Settings</button>
      <hr>
      <button @click="onSave" class="largeButton">Download save data</button>
      <input @change="onLoadFileChange" style="display: none" type="file" ref="loadFile">
      <button @click="onLoad" class="largeButton">Load saved game</button>
      <input ref="saveText" type="text" value="" style="display: none">
      <hr>
      <div>Source on <a href="https://github.com/msakuta/FactorishWasm">GitHub</a>.</div>
    </div>
  </div>
</template>
