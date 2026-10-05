interface bus_if #(parameter W = 8) ();
  logic [W-1:0] d;
endinterface
module top;
  localparam integer K = 1;
  if (1) begin : g
    if (K == 2) begin : a
      bus_if #(.W(8)) u();
    end else begin : b
      bus_if #(.W(4)) u();
    end
    localparam integer K = 2;
  end
  initial #1 $display("@bits=%0d", $bits(g.b.u.d));
  initial #10 $finish;
endmodule
