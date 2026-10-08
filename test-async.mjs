async function main() {
  console.log("start");
  await Promise.resolve();
  console.log("end");
}
main().catch(console.error);
