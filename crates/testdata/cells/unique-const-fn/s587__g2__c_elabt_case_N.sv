module top;
  function automatic int f(input int a);
    f = 7;
    unique case (a) 1: f = 10; 3: f = 30; endcase
  endfunction
  if (1) begin : g
    $info("el=%0d", f(1));
  end
  initial begin #1 $finish; end
endmodule
