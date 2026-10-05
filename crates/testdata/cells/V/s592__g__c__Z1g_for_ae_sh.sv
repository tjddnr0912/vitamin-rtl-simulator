module top;
  function automatic logic [3:0] fx(input int a); logic [3:0] t; fx = t; endfunction
  localparam N = 1;
  if (1) begin : gb
    for (genvar i = 0; i < N; i++) begin : L
      localparam logic [3:0] P = fx(i);
      wire [7:0] w = 8'd10 + i;
      initial #1 $display("@L%0d w=%0d P=%b", i, w, P);
    end
    localparam N = 3;
  end
  initial #5 $finish;
endmodule
