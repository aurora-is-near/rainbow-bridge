const { join } = require("path");
const { readdirSync, statSync } = require("fs");

function getAllJsonFilesRecursive(dirPath) {
	let arrayOfFiles = [];
	const files = readdirSync(dirPath);

	files.forEach((file) => {
		if (statSync(join(dirPath, file)).isDirectory()) {
			const subDirFiles = getAllJsonFilesRecursive(join(dirPath, file));
			arrayOfFiles = arrayOfFiles.concat(subDirFiles);
		} else if (file.endsWith(".json")) {
			arrayOfFiles.push(join(dirPath, file));
		}
	});

	return arrayOfFiles;
}

module.exports = {
	getAllJsonFilesRecursive,
}