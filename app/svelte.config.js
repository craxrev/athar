import adapter from '@sveltejs/adapter-static';

/** @type {import('@sveltejs/kit').Config} */
export default {
  kit: {
    // A desktop shell: no server, everything prerendered and loaded from disk.
    adapter: adapter({ fallback: 'index.html' })
  }
};
