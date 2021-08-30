<script>
import closeImage from "../../img/close.png";

export default {
  name: 'CloseButton',

  setup(props, context) {
    return {
      closeImage,
    }
  }
}
</script>

<template>
  <div class="closeButton" :style="{backgroundImage: `url(${closeImage})`}"></div>
</template>

<style scoped>
div.closeButton{
    position: absolute;
    width: 16px;
    height: 16px;
    right: 2px;
    top: 2px;
    border: 1px solid #7f7f7f;
}
</style>