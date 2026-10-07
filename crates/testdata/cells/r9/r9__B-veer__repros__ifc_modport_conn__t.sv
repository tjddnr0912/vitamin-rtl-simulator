interface bus_if;
  logic [7:0] d;
  logic [7:0] q;
  modport src (output d, input q);
endinterface
module leaf (bus_if.src b);
  assign b.d = b.q + 8'd1;
endmodule
module top;
  bus_if bi ();
  leaf u (.b(bi.src));          // modport expression in the connection (IEEE 1800-2017 25.5)
  initial begin
    bi.q = 8'd41;
    #1 $display("d=%0d", bi.d);
    $finish;
  end
endmodule
