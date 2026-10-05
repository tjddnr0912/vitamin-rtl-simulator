module top;
  wire [7:0] m = f(8'd5);
  function logic [7:0] f(input logic [7:0] x);
    f = x + $random;
  endfunction
  initial begin
    #1 $display("t=%0t m=%h", $time, m);
    #1 $display("t=%0t m=%h", $time, m);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
