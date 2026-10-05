module child #(parameter logic [3:0] P = 4'd9) ();
  initial begin #1 $display("P=%b", P); end
endmodule
module top;
  function automatic logic [3:0] fx(input int a);
    logic [3:0] t;
    case (a) 1: t = 4'd5; endcase
    fx = t;
  endfunction
  child #(.P(fx(2))) u();
  initial #100 $finish;
endmodule
