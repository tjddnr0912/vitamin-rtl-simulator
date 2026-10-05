module child(input logic i, output logic o);
  assign o = i ^ 1'b0;
endmodule
module top;
  logic a;
  function automatic logic chk(input logic b);
    if (b === 1'bz) $display("never");
    chk = ~b;
  endfunction
  function automatic logic g(input logic x);
    $display("g t=%0t x=%b", $time, x);
    g = x;
  endfunction
  wire h = chk(a);
  wire p = g(h);
  wire q = h ^ 1'b0;
  wire r = q & 1'b1;
  wire oq;
  child u(.i(h), .o(oq));
  initial $display("init h=%b p=%b q=%b r=%b oq=%b", h, p, q, r, oq);
  always @(q) $display("ev q=%b t=%0t", q, $time);
  always @(posedge r) $display("pos r t=%0t", $time);
  always @(oq) $display("ev oq=%b t=%0t", oq, $time);
  initial begin a = 0; #1 $display("t1 h=%b p=%b q=%b r=%b oq=%b", h, p, q, r, oq); $finish; end
  initial #10 $finish;
endmodule
