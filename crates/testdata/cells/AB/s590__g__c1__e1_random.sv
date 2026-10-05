module top;
  logic [7:0] a; wire [7:0] m;
  function logic [7:0] f(input logic [7:0] x);
    f = x + $random;
  endfunction
  assign m = f(a);
  initial begin
    a = 8'd5;
    #1 $display("t=%0t m=%h", $time, m);
    #1 $display("t=%0t m=%h", $time, m);
    a = 8'd6;
    #1 $display("t=%0t m=%h", $time, m);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
