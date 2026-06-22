use crate::prog_settings::MonitorTarget;
use crate::prog_settings::CommonSettings;

use lettre::message::{Mailbox, header::ContentType};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use lettre::transport::smtp::client::{TlsParametersBuilder, Tls};

pub type StError = <SmtpTransport as Transport>::Error;

pub async fn report_by_email(line: &str, title_prefix: &str, target_name: &str, target: &MonitorTarget, common_settings: &CommonSettings) -> Result<(), StError>{
    if let None = common_settings.report_from {
        return Ok(());
    }

    if let None = common_settings.report_to {
        return Ok(());
    }

    if let None = common_settings.smtp_host {
        return Ok(());
    }

    if let None = common_settings.smtp_port {
        return Ok(());
    }

    let report_from = common_settings.report_from.as_ref().unwrap();
    let report_to = common_settings.report_to.as_ref().unwrap();
    let email = Message::builder()
        .from(
            Mailbox::new(
                None, 
                report_from.as_str().parse().unwrap()
            )
        )
        .reply_to(
            Mailbox::new(
                None, 
                report_from.as_str().parse().unwrap()
            )
        )
        .to(
            Mailbox::new(
                None, 
                report_to.as_str().parse().unwrap()
            )
        )
        .subject(
            format!("{}{}", title_prefix, target_name)
        )
        .header(ContentType::TEXT_PLAIN)
        .body(format!("{}\n\nOn target: {}\n{:#?}", line.to_owned(), target_name, target))
        .unwrap();

    let mut mailer_builder = match common_settings.smtp_starttls {
        true => SmtpTransport::starttls_relay(common_settings.smtp_host.as_ref().unwrap().as_str()).unwrap(),
        false => SmtpTransport::relay(common_settings.smtp_host.as_ref().unwrap().as_str()).unwrap(),
    };

    let mut tls_builder = TlsParametersBuilder::new(common_settings.smtp_host.as_ref().unwrap().clone());
    if common_settings.smtp_no_verify_hostname {
        tls_builder = tls_builder.dangerous_accept_invalid_hostnames(true);
    }
    if common_settings.smtp_no_check_certificate {
        tls_builder = tls_builder.dangerous_accept_invalid_certs(true);
    }
    mailer_builder = mailer_builder.tls(Tls::Required(tls_builder.build()?));

    if let Some(smtp_user) = &common_settings.smtp_user {
        if let Some(smtp_pass) = &common_settings.smtp_pass {
            let creds = Credentials::new(
                smtp_user.clone(),
                smtp_pass.clone()
            );
            mailer_builder = mailer_builder.credentials(creds);
        }
    }
    
    let mailer = mailer_builder.build();
    match mailer.send(&email) {
        Ok(_) => Ok(()),
        Err(err) => Err(err)
    }
}