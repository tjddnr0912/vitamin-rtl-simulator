module top;
  function automatic logic [3:0] fx(input int a);
    logic [3:0] t;
    case (a) 1: t = 4'd5; endcase
    fx = t;
  endfunction
  localparam logic P = (fx(2) === 4'bxxxx);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
