use std::fmt::Write as _;

pub const COMPLETE_EDIT: &str = r#"const selectedService: Service = selectService({
    name: "editor",
    status: "online",
    endpoint: { host: "editor.internal", port: 9000 },
    retryDelays: [100, 250, 500],
});
selectService(selectedService);"#;

pub const MISSING_PROPERTY_VALUE_EDIT: &str = r#"const selectedService: Service = selectService({
    name: "editor",
    status: "online",
    endpoint: { host: "editor.internal", port:  },
    retryDelays: [100, 250, 500],
});
selectService(selectedService);"#;

pub const MISSING_CALL_CLOSER_EDIT: &str = r#"const selectedService: Service = selectService({
    name: "editor",
    status: "online",
    endpoint: { host: "editor.internal", port: 9000 },
    retryDelays: [100, 250, 500],
});
selectService("#;

pub const MISSING_DECLARATION_NAME_EDIT: &str = r#"const selectedService: Service = selectService({
    name: "editor",
    status: "online",
    endpoint: { host: "editor.internal", port: 9000 },
    retryDelays: [100, 250, 500],
});
selectService(selectedService);
const = 1;"#;

const SERVICES_PER_SIDE: usize = 96;

pub fn editor_trace_source(edit: &str) -> String {
    let mut source = String::from(
        r#"type ServiceStatus = "online" | "offline" | "starting";

interface Endpoint {
    host: string;
    port: number;
}

interface Service {
    name: string;
    status: ServiceStatus;
    endpoint: Endpoint;
    retryDelays: number[];
}

function selectService(service: Service): Service {
    return service;
}

const firstSentinel: number = "persistent before edit";

"#,
    );

    append_services(&mut source, "before", 0);
    source.push_str(edit);
    source.push_str("\n\n");
    append_services(&mut source, "after", SERVICES_PER_SIDE);
    source.push_str("const finalSentinel: number = \"persistent after edit\";\n");
    source
}

fn append_services(source: &mut String, group: &str, index_offset: usize) {
    for index in 0..SERVICES_PER_SIDE {
        let service_number = index + index_offset;
        writeln!(
            source,
            r#"const service{service_number:03}: Service = {{
    name: "{group}-{service_number:03}",
    status: "online",
    endpoint: {{ host: "{group}-{service_number:03}.internal", port: {} }},
    retryDelays: [100, 250, 500],
}};
"#,
            8000 + service_number,
        )
        .expect("writing to a String cannot fail");
    }
}
