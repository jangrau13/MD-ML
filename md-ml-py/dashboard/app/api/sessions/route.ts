import { NextRequest, NextResponse } from "next/server";
import { listSessions, createSession } from "@/lib/db";

export async function GET() {
  return NextResponse.json(listSessions());
}

export async function POST(req: NextRequest) {
  const body = await req.json();
  const id = createSession(body);
  return NextResponse.json({ id });
}
