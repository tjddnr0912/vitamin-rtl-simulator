module top;
  function automatic integer n(input integer x); n = x + 2; endfunction
  if (1) begin : g
    for (genvar i = 0; i < n(1); i++) begin : L
      initial $display("@L%0d", i);
    end
    function automatic integer n(input integer x); n = x; endfunction
  end
  initial #10 $finish;
endmodule
