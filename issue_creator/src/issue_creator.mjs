import { Octokit } from '@octokit/rest';
import fs from 'fs/promises';
import cliProgress from 'cli-progress';
import yargs from 'yargs';
import { hideBin } from 'yargs/helpers';
import _ from 'lodash';
import path from 'path';

// Parse command-line arguments
const argv = yargs(hideBin(process.argv))
    .option("token", {
        alias: "t",
        description: "GitHub token",
        type: "string",
    })
    .option("repos", {
        alias: "r",
        description: "Path to repositories JSON file",
        type: "string",
        default: "repositories.json",
    })
    .option("issue", {
        alias: "i",
        description: "Path to issue content Markdown file",
        type: "string",
        default: "issue_content.md",
    })
    .option("progress", {
        alias: "p",
        description: "Path to progress JSON file",
        type: "string",
        default: "progress.json",
    })
    .option("limit", {
        alias: "l",
        description: "Limit the number of issues to create",
        type: "number",
    })
    .option("generate", {
        alias: "g",
        description: "Generate hand-selected repositories list",
        type: "boolean",
    })
    .option("output", {
        alias: "o",
        description: "Output file for hand-selected repositories",
        type: "string",
        default: "hand_selected.json",
    })
    .option("stats", {
        alias: "s",
        description: "Calculate and display statistics",
        type: "boolean",
    })
    .help()
    .alias("help", "h")
    .parse();

// Configuration
const config = {
    githubToken: argv.token || process.env.GITHUB_TOKEN,
    repositoriesFile: argv.repos,
    issueContentFile: argv.issue,
    progressFile: argv.progress,
    rateLimit: {
        perHour: 10,
    },
    maxRetries: 3,
    retryDelay: 5000,
};

const octokit = new Octokit({ auth: config.githubToken });

function parseRepositoryString(repoString) {
    // Remove the "https://github.com/" prefix if it exists
    const cleanedString = repoString.replace(/^https:\/\/github\.com\//, '');

    // Split the remaining string into owner and repo
    const [owner, repo] = cleanedString.split('/');

    if (!owner || !repo) {
        throw new Error(`Invalid repository format: ${repoString}`);
    }

    return { owner, repo };
}

function getOrgFromRepo(repoString) {
    const { owner } = parseRepositoryString(repoString);
    return owner;
}

async function loadRepositories() {
    const data = await fs.readFile(config.repositoriesFile, "utf8");
    return JSON.parse(data);
}

async function loadIssueContent() {
    const data = await fs.readFile(config.issueContentFile, "utf8");
    const lines = data.split("\n");
    const title = lines[0].replace(/^#\s*/, "").trim(); // Remove leading # if present
    const body = lines.slice(1).join("\n").trim();
    return { title, body };
}

async function loadProgress() {
    try {
        const data = await fs.readFile(config.progressFile, "utf8");
        return JSON.parse(data);
    } catch (error) {
        return {};
    }
}

async function saveProgress(progress) {
    await fs.writeFile(config.progressFile, JSON.stringify(progress, null, 2));
}

async function createIssueWithRetry(owner, repo, issueContent, retries = 0) {
    try {
        const response = await octokit.issues.create({
            owner,
            repo,
            title: issueContent.title,
            body: issueContent.body,
        });
        console.log(`Issue created in ${owner}/${repo}: ${response.data.html_url}`);
        return response.data.html_url;
    } catch (error) {
        console.error(`Error creating issue in ${owner}/${repo}:`, error.message);
        if (retries < config.maxRetries) {
            console.log(`Retrying in ${config.retryDelay / 1000} seconds... (Attempt ${retries + 1} of ${config.maxRetries})`);
            await new Promise(resolve => setTimeout(resolve, config.retryDelay));
            return createIssueWithRetry(owner, repo, issueContent, retries + 1);
        }
        return { error: error.message };
    }
}

async function generateHandSelectedList(repositories, progress, limit) {
    const processedOrgs = new Set(
        Object.keys(progress)
            .filter(repo => progress[repo] && !progress[repo].error)
            .map(getOrgFromRepo)
    );

    const unprocessedRepos = repositories.filter(repo => {
        const org = getOrgFromRepo(repo);
        return !processedOrgs.has(org) && (!progress[repo] || progress[repo].error);
    });

    const reposByOrg = _.groupBy(unprocessedRepos, getOrgFromRepo);

    // Select one random repo from each unprocessed org
    const selectedRepos = Object.values(reposByOrg)
        .map(orgRepos => _.sample(orgRepos));

    // Shuffle the selected repos and limit to the specified number
    const finalSelectedRepos = _.sampleSize(selectedRepos, limit || selectedRepos.length);

    return finalSelectedRepos;
}

async function calculateStats(repositories, progress) {
    const processedRepos = new Set(Object.keys(progress));
    const processedOrgs = new Set(
        Object.keys(progress).map(getOrgFromRepo)
    );

    const uniqueRepos = repositories.filter(repo => !processedRepos.has(repo));
    const uniqueOrgs = new Set(
        repositories
            .filter(repo => !processedOrgs.has(getOrgFromRepo(repo)))
            .map(getOrgFromRepo)
    );

    return {
        uniqueRepos: uniqueRepos.length,
        uniqueOrgs: uniqueOrgs.size,
        totalRepos: repositories.length,
        processedRepos: processedRepos.size,
        processedOrgs: processedOrgs.size
    };
}

async function main() {
    if (!config.githubToken) {
        console.error("GitHub token is required. Set it using the --token option or GITHUB_TOKEN environment variable.");
        process.exit(1);
    }

    const repositories = await loadRepositories();
    let progress = await loadProgress();

    if (argv.stats) {
        const stats = await calculateStats(repositories, progress);
        console.log("\nStatistics:");
        console.log(`Total repositories: ${stats.totalRepos}`);
        console.log(`Processed repositories: ${stats.processedRepos}`);
        console.log(`Unique repositories left: ${stats.uniqueRepos}`);
        console.log(`Processed organizations: ${stats.processedOrgs}`);
        console.log(`Unique organizations left: ${stats.uniqueOrgs}`);
        return;
    }

    const issueContent = await loadIssueContent();

    // Create a multi-bar progress display
    const multibar = new cliProgress.MultiBar({
        clearOnComplete: false,
        hideCursor: true,
        format: ' {bar} | {percentage}% | {value}/{total} | {task}',
    }, cliProgress.Presets.shades_classic);

    if (argv.generate) {
        const handSelectedRepos = await generateHandSelectedList(repositories, progress, argv.limit);
        const outputPath = path.resolve(argv.output);
        await fs.writeFile(outputPath, JSON.stringify(handSelectedRepos, null, 2));
        console.log(`Generated hand-selected list with ${handSelectedRepos.length} repositories. Saved to ${outputPath}`);
        return;
    }

    let repositoriesToProcess = repositories;
    if (argv.limit) {
        // Shuffle the selected repos and limit to the specified number
        repositoriesToProcess = await generateHandSelectedList(repositories, progress, argv.limit);

        console.log(`Processing ${repositoriesToProcess.length} repositories`);

        // Pre-log repositories to be processed
        console.log("\nRepositories to be processed:");
        repositoriesToProcess.forEach(repo => {
            console.log(`- ${repo}`);
        });
        console.log(); // Add an empty line for better readability
    }

    const overallBar = multibar.create(repositoriesToProcess.length, 0, { task: "Overall Progress" });
    const hourRateBar = multibar.create(config.rateLimit.perHour, 0, { task: "Hour Rate Limit" });

    let hourCount = 0;
    let lastHour = Math.floor(Date.now() / 3600000);

    overallBar.update(Object.keys(progress).length);

    for (const repoString of repositoriesToProcess) {
        if (progress[repoString] && !progress[repoString].error) {
            console.log(`Skipping ${repoString}, already processed successfully.`);
            continue;
        }

        try {
            const { owner, repo } = parseRepositoryString(repoString);
            const result = await createIssueWithRetry(owner, repo, issueContent);

            progress[repoString] = result;
            await saveProgress(progress);
            overallBar.increment();
        } catch (error) {
            console.error(`Error processing ${repoString}:`, error.message);
            progress[repoString] = { error: error.message };
            await saveProgress(progress);
            overallBar.increment();
        }

        // Update rate limit bar
        const currentHour = Math.floor(Date.now() / 3600000);

        if (currentHour !== lastHour) {
            hourCount = 0;
            hourRateBar.update(0);
            lastHour = currentHour;
        }

        hourCount++;
        hourRateBar.update(hourCount);

        // Respect rate limit
        await new Promise(resolve => setTimeout(resolve, 3600000 / config.rateLimit.perHour));
    }

    multibar.stop();
    console.log("Process completed. Check progress.json for results.");

    // Print summary
    const successful = Object.values(progress).filter(v => !v.error).length;
    const failed = Object.values(progress).filter(v => v.error).length;
    console.log(`\nSummary:`);
    console.log(`Total repositories: ${repositories.length}`);
    console.log(`Successful: ${successful}`);
    console.log(`Failed: ${failed}`);

    if (failed > 0) {
        console.log(`\nFailed repositories:`);
        for (const [repo, result] of Object.entries(progress)) {
            if (result.error) {
                console.log(`${repo}: ${result.error}`);
            }
        }
    }
}

main().catch(console.error);
