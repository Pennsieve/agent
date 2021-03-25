require 'open3'


# Jekyll plugin that documents all available agent commands
#
# Creates a help page and an entry in the TOC for each command and subcommand

HEADERS = [
  :SUMMARY,
  :USAGE,
  :OPTIONS,
  :FLAGS,
  :ARGS,
  :SUBCOMMANDS,
]

module Jekyll
  class AgentCommandPageGenerator < Generator
    safe true

    def generate(site)
      # Inject the version of the agent
      site.data['agent_version'] = agent_version()

      find_subcommands().each do |command|
        site.pages << AgentCommandPage.new(site, site.source, @dir, command)
      end
    end
  end

  # A page generated for an agent subcommand
  class AgentCommandPage < Page

    def initialize(site, base, dir, command)
      puts "Generating page for #{command}"

      @site = site
      @base = base
      @dir = dir

      # Join subcommands into a valid filename
      command_slug = 'agent_' + Jekyll::Utils::slugify(command).gsub('-', '_')
      @name = "#{command_slug}.md"
      @permalink = "/#{command_slug}.html"

      # Initialize page
      self.process(@name)

      # Capture help text from agent
      sections = parse_docstring(agent_help(command))

      # Set up the front matter variables for the page.html template
      self.data ||= {
        'layout' => 'page',
        'sidebar'=> 'agent_sidebar',
        'keywords' => 'pennsieve agent command',
        'folder' => 'agent',
        'title' => "Command - #{command}",
        'summary' => sections[:SUMMARY]
      }

      # Generate page content
      # Dump help sections into Markdown code block and convert to HTML
      markdown = ""
      for header in HEADERS
        if (sections.has_key? header) && (header != :SUMMARY)
          markdown << "### #{header.capitalize}\n```\n#{sections[header]}\n```\n"
        end
      end
      converter = site.find_converter_instance(::Jekyll::Converters::Markdown)
      self.content = converter.convert markdown

      # Generate a sidebar entry for each command
      # NOTE: this is highly dependent on the structure of `agent_sidebar.yml`
      commands_sidebar =
        site.data['sidebars']['agent_sidebar']['entries']
          .detect{|s| s['title'] == "Developer docs"}['folders']
          .detect{|s| s['title'] == "Commands"}

      if !commands_sidebar.has_key? 'folderitems'
        commands_sidebar['folderitems'] = []
      end

      commands_sidebar['folderitems'] << {
        "title" => command,
        "url" => @permalink,
        "output" => "web"
      }
    end

  end
end


# Recursively build a list of agent subcommands, eg
# ['profile', 'profile create', 'profile delete', ...]
def find_subcommands(command = "")

  # Parse subcommands from help text
  sections = parse_docstring(agent_help(command))

  if !sections.has_key? :SUBCOMMANDS
    return []
  end

  sections[:SUBCOMMANDS].strip.split("\n")
    .map {|line| /[[:graph:]]+/.match(line)[0] }  # Commands are first word on each line
    .select {|subcommand| subcommand != "help" }  # We don't care about help commands
    .map {|subcommand| "#{command} #{subcommand}".strip}  # Concatenate base and sub
    .flat_map {|subcommand| [subcommand] + find_subcommands(subcommand)}
end


# Parse an agent docstring into component parts
def parse_docstring(buf)
  lines = buf.split("\n")

  # An un-indented header marks the beginning of each section
  # Convert those into our HEADER token
  regions = lines
    .map {|line| HEADERS.select {|h| line.start_with?(h.to_s)}.first || line }
    .slice_before {|line| HEADERS.include? line}

  # Remove CLI invocation on first line and extract the command description
  # as the summary.
  # NOTE: this may need to change in the future if commands descriptions
  # become longer - we might need to parse the "short" command, eg in the
  # SUBCOMMANDS section.
  summary = regions.first[1..-1].join("\n")

  # Accumulate the content of each section, stripping 4 chars of indentation
  sections = {:SUMMARY => summary}
  regions
    .drop(1)
    .each { |region|
      section_header, *section_lines = *region  # head/tail == section/content
      sections[section_header] = section_lines.map {
        |line| line[4..-1]}.join("\n").rstrip
    }
  return sections
end


def agent_help(command)
  _, stderr, _ = agent("help #{command}")
  stderr
end


def agent_version()
  stdout, _, _ = agent("version")
  stdout.strip()
end


# Call out to the agent
# NOTE: this currently searches for a release build of the agent
def agent(command)
  executable = File.join(__dir__, "../../target/release/pennsieve")
  Open3.capture3("#{executable} #{command}")
end
