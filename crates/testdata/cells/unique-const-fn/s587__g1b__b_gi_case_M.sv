module top;
  function automatic int f(input int a);
    f = 7;
    unique case (a) 1: f = 10; 3: f = 30; endcase
  endfunction
  if (f(2) == 7) begin : g7
    initial begin #1 $display("gi=7"); $finish; end
  end else begin : gx
    initial begin #1 $display("gi=other"); $finish; end
  end
endmodule
