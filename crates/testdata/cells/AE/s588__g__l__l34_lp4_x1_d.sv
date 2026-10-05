module top;
  function automatic logic [3:0] fx1(input int a);
    logic [3:0] t;
    t[0] = 1'b1;
    case (a) 1: t = 4'd5; endcase
    fx1 = t;
  endfunction
  localparam logic [3:0] P = fx1(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
