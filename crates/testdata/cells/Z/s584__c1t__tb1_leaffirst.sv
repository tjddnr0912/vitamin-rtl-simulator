module leaf(input logic [1:0] s, output logic [1:0] y);
  always_comb begin
    $display("C t=%0t s=%b", $time, s);
    y = 0;
    unique case (s) 2'd1: y = 1; 2'd2: y = 2; endcase
  end
endmodule
module top;
  logic [1:0] s, y;
  leaf u(.s(s), .y(y));
  initial begin
    $display("I0 t=%0t s=%b u.s=%b", $time, s, u.s);
    s = 2'd1;
    $display("I1 t=%0t s=%b u.s=%b", $time, s, u.s);
    #0 $display("I2 t=%0t s=%b u.s=%b", $time, s, u.s);
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
