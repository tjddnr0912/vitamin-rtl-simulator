interface bus_if;
  logic [1:0] req;
  logic [1:0] gnt;
  modport m (output req, input gnt);
  modport s (input req, output gnt);
endinterface
module arb(bus_if.s b);
  always_comb b.gnt = b.req ^ 2'b11;
endmodule
module top;
  bus_if b();
  arb u(.b(b));
  always @(b.gnt) $display("G t=%0t gnt=%b", $time, b.gnt);
  initial begin b.req = 2'b01; #1 $display("t=%0t gnt=%b", $time, b.gnt); end
  initial #10 $finish;
endmodule
