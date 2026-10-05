module top;
  logic [3:0] r = 4'd5;
  wire [3:0] k;
  assign k = 4'd6;
  function automatic logic f1(input logic [3:0] x);
    if (x === 4'bzzzz) $display("never");
    f1 = (x == 4'd5) || (x == 4'd6);
  endfunction
  wire p1 = f1(r);
  wire p2 = f1(k);
  logic q1;
  assign q1 = f1(k);
  initial begin @(p1) $display("ev p1=%b t=%0t", p1, $time); end
  initial begin @(p2) $display("ev p2=%b t=%0t", p2, $time); end
  always @(posedge p1) $display("pos p1 t=%0t", $time);
  always @(posedge p2) $display("pos p2 t=%0t", $time);
  always @(posedge q1) $display("pos q1 t=%0t", $time);
  initial $display("init p1=%b p2=%b q1=%b", p1, p2, q1);
  initial #3 $finish;
endmodule
